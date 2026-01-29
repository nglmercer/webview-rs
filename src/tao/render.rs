//! Software rendering support for tao windows using softbuffer
//!
//! This module provides pixel buffer rendering capabilities for tao windows,
//! allowing direct pixel manipulation without requiring a webview.

use std::collections::HashMap;
use std::mem::ManuallyDrop;
use std::num::NonZeroU32;
use std::sync::Mutex;

/// Combined storage for context and surface to preserve their internal relationship
/// The surface borrows from the context, so they must be stored together
#[allow(dead_code)]
struct GraphicsResources<'a> {
  context: ManuallyDrop<softbuffer::Context<&'a tao::window::Window>>,
  surface: ManuallyDrop<softbuffer::Surface<&'a tao::window::Window, &'a tao::window::Window>>,
}

// SAFETY: GraphicsResources contains thread-safe softbuffer types
unsafe impl Send for GraphicsResources<'_> {}
unsafe impl Sync for GraphicsResources<'_> {}

/// Cached surface entry for a window using raw pointers
/// This avoids lifetime issues with the window reference
struct CachedSurface {
  resources_ptr: *mut (),
  width: u32,
  height: u32,
}

// SAFETY: We ensure the pointers are only accessed when valid
unsafe impl Send for CachedSurface {}
unsafe impl Sync for CachedSurface {}

/// Global storage for cached softbuffer surfaces (window_id -> cached surface)
/// This prevents "Maximum number of clients reached" errors by reusing connections
static CACHED_SURFACES: Mutex<Option<HashMap<u64, CachedSurface>>> = Mutex::new(None);

/// Initialize the cached surface storage
fn init_surface_storage() {
  let mut storage = CACHED_SURFACES.lock().unwrap();
  if storage.is_none() {
    *storage = Some(HashMap::new());
  }
}

/// Remove cached surface when window is closed
pub fn remove_render_surface(window_id: u64) {
  let mut storage = CACHED_SURFACES.lock().unwrap();
  if let Some(map) = storage.as_mut() {
    if let Some(entry) = map.remove(&window_id) {
      // SAFETY: We own this pointer and are cleaning it up
      unsafe {
        if !entry.resources_ptr.is_null() {
          let _ = Box::from_raw(entry.resources_ptr as *mut GraphicsResources);
        }
      }
    }
  }
}

/// Render pixel buffer to a window
/// The buffer should be in RGBA format
/// This caches the softbuffer surface to avoid creating new display connections on each render
pub fn render_pixels(
  window: &tao::window::Window,
  window_id: u64,
  width: u32,
  height: u32,
  rgba_buffer: &[u8],
) -> napi::Result<()> {
  // Validate dimensions first
  if width == 0 || height == 0 {
    return Err(napi::Error::new(
      napi::Status::GenericFailure,
      format!("Invalid dimensions: {}x{}", width, height),
    ));
  }

  let width_nz = NonZeroU32::new(width)
    .ok_or_else(|| napi::Error::new(napi::Status::GenericFailure, "Invalid width"))?;
  let height_nz = NonZeroU32::new(height)
    .ok_or_else(|| napi::Error::new(napi::Status::GenericFailure, "Invalid height"))?;

  init_surface_storage();

  let mut storage = CACHED_SURFACES.lock().unwrap();

  if let Some(map) = storage.as_mut() {
    // Check if we need to create or recreate the surface
    let needs_create = if let Some(entry) = map.get(&window_id) {
      entry.width != width || entry.height != height
    } else {
      true
    };

    if needs_create {
      // Clean up old surface if it exists
      if let Some(old_entry) = map.remove(&window_id) {
        unsafe {
          if !old_entry.resources_ptr.is_null() {
            let _ = Box::from_raw(old_entry.resources_ptr as *mut GraphicsResources);
          }
        }
      }

      // Create new context and surface
      let context = softbuffer::Context::new(window).map_err(|e| {
        napi::Error::new(
          napi::Status::GenericFailure,
          format!("Failed to create softbuffer context: {:?}", e),
        )
      })?;

      let mut surface = softbuffer::Surface::new(&context, window).map_err(|e| {
        napi::Error::new(
          napi::Status::GenericFailure,
          format!("Failed to create softbuffer surface: {:?}", e),
        )
      })?;

      // IMPORTANT: Must set surface size before calling buffer_mut()
      // This is required by softbuffer's Wayland backend
      // Resize the surface immediately after creation
      surface.resize(width_nz, height_nz).map_err(|e| {
        napi::Error::new(
          napi::Status::GenericFailure,
          format!("Failed to resize surface: {:?}", e),
        )
      })?;

      // Create the GraphicsResources with owned types using ManuallyDrop
      // This prevents the values from being dropped when we access them
      let resources = Box::new(GraphicsResources {
        context: ManuallyDrop::new(context),
        surface: ManuallyDrop::new(surface),
      });

      map.insert(
        window_id,
        CachedSurface {
          resources_ptr: Box::into_raw(resources) as *mut (),
          width,
          height,
        },
      );
    }

    // Get mutable reference to cached surface
    if let Some(entry) = map.get_mut(&window_id) {
      // SAFETY: We reconstruct the Box to get a mutable reference
      // ManuallyDrop prevents double-drop when we access the fields
      let resources = unsafe { &mut *(entry.resources_ptr as *mut GraphicsResources) };
      // We need to use the surface through a pointer to avoid borrowing issues
      let surface_ptr = &mut *resources.surface
        as *mut softbuffer::Surface<&tao::window::Window, &tao::window::Window>;
      let surface = unsafe { &mut *surface_ptr };

      // IMPORTANT: On Wayland, we must call resize before buffer_mut()
      // The surface needs to know its size before we can get a buffer.
      // We always call resize because on Wayland the size state can be lost
      // between frames if the compositor releases the buffer.
      surface.resize(width_nz, height_nz).map_err(|e| {
        napi::Error::new(
          napi::Status::GenericFailure,
          format!("Failed to resize surface: {:?}", e),
        )
      })?;

      // Update cached dimensions
      entry.width = width;
      entry.height = height;

      // Get the surface buffer and fill with pixel data
      let mut surface_buffer = surface.buffer_mut().map_err(|e| {
        napi::Error::new(
          napi::Status::GenericFailure,
          format!("Failed to get surface buffer: {:?}", e),
        )
      })?;

      // Check buffer length to avoid zero-size buffer issues
      let buffer_len = surface_buffer.len();
      if buffer_len == 0 {
        return Err(napi::Error::new(
          napi::Status::GenericFailure,
          "Surface buffer has zero size",
        ));
      }

      // Convert RGBA to ARGB (softbuffer format)
      // softbuffer uses 0xAARRGGBB format
      let len = (width * height) as usize;
      if rgba_buffer.len() >= len * 4 {
        for i in 0..len.min(buffer_len) {
          let idx = i * 4;
          let r = rgba_buffer[idx] as u32;
          let g = rgba_buffer[idx + 1] as u32;
          let b = rgba_buffer[idx + 2] as u32;
          let a = rgba_buffer[idx + 3] as u32;
          // ARGB format
          surface_buffer[i] = (a << 24) | (r << 16) | (g << 8) | b;
        }
      }

      // Present the buffer to the window
      surface_buffer.present().map_err(|e| {
        napi::Error::new(
          napi::Status::GenericFailure,
          format!("Failed to present buffer: {:?}", e),
        )
      })?;
    }
  }

  Ok(())
}
