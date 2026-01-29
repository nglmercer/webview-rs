//! Software rendering support for tao windows using softbuffer
//!
//! This module provides pixel buffer rendering capabilities for tao windows,
//! allowing direct pixel manipulation without requiring a webview.

use crate::tao::enums::ScaleMode;
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
  // Get the actual window size for rendering
  let window_size = window.inner_size();
  let options = RenderOptions {
    buffer_width: width,
    buffer_height: height,
    window_width: window_size.width,
    window_height: window_size.height,
    auto_scale: false,
    scale_mode: ScaleMode::Stretch,
    background_color: [0, 0, 0, 255],
  };
  render_pixels_scaled(window, window_id, options, rgba_buffer)
}

/// Render options for scaling
pub struct RenderOptions {
  /// Width of the source pixel buffer
  pub buffer_width: u32,
  /// Height of the source pixel buffer
  pub buffer_height: u32,
  /// Current width of the window (physical pixels)
  pub window_width: u32,
  /// Current height of the window (physical pixels)
  pub window_height: u32,
  /// Whether to apply scaling
  pub auto_scale: bool,
  /// How to scale the buffer (Stretch, Fit, Fill, Integer, None)
  pub scale_mode: ScaleMode,
  /// Background color for letterboxing [R, G, B, A]
  pub background_color: [u8; 4],
}

/// Render pixel buffer to a window with auto-scaling support
/// The buffer should be in RGBA format
/// This caches the softbuffer surface to avoid creating new display connections on each render
pub fn render_pixels_scaled(
  window: &tao::window::Window,
  window_id: u64,
  options: RenderOptions,
  rgba_buffer: &[u8],
) -> napi::Result<()> {
  let RenderOptions {
    buffer_width,
    buffer_height,
    window_width,
    window_height,
    auto_scale,
    scale_mode,
    background_color,
  } = options;
  // Validate dimensions first
  if buffer_width == 0 || buffer_height == 0 {
    return Err(napi::Error::new(
      napi::Status::GenericFailure,
      format!(
        "Invalid buffer dimensions: {}x{}",
        buffer_width, buffer_height
      ),
    ));
  }

  if window_width == 0 || window_height == 0 {
    return Err(napi::Error::new(
      napi::Status::GenericFailure,
      format!(
        "Invalid window dimensions: {}x{}",
        window_width, window_height
      ),
    ));
  }

  let window_width_nz = NonZeroU32::new(window_width)
    .ok_or_else(|| napi::Error::new(napi::Status::GenericFailure, "Invalid window width"))?;
  let window_height_nz = NonZeroU32::new(window_height)
    .ok_or_else(|| napi::Error::new(napi::Status::GenericFailure, "Invalid window height"))?;

  init_surface_storage();

  let mut storage = CACHED_SURFACES.lock().unwrap();

  if let Some(map) = storage.as_mut() {
    // Check if we need to create or recreate the surface
    // Surface is keyed by window_id and window dimensions
    let needs_create = if let Some(entry) = map.get(&window_id) {
      entry.width != window_width || entry.height != window_height
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
      surface
        .resize(window_width_nz, window_height_nz)
        .map_err(|e| {
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
          width: window_width,
          height: window_height,
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
      surface
        .resize(window_width_nz, window_height_nz)
        .map_err(|e| {
          napi::Error::new(
            napi::Status::GenericFailure,
            format!("Failed to resize surface: {:?}", e),
          )
        })?;

      // Update cached dimensions
      entry.width = window_width;
      entry.height = window_height;

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

      // Calculate scaled dimensions and render
      let scaled = calculate_scaled_dimensions(
        buffer_width,
        buffer_height,
        window_width,
        window_height,
        auto_scale,
        &scale_mode,
      );

      // Fill background if needed
      if scaled.needs_fill {
        fill_background(
          &mut surface_buffer,
          window_width,
          window_height,
          background_color,
        );
      }

      // Scale and copy pixel data
      scale_and_copy_pixels(
        &mut surface_buffer,
        rgba_buffer,
        buffer_width,
        buffer_height,
        window_width,
        &scaled,
      );

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

/// Scaled dimensions for rendering
struct ScaledDimensions {
  src_x: u32,
  src_y: u32,
  src_width: u32,
  src_height: u32,
  dst_x: u32,
  dst_y: u32,
  dst_width: u32,
  dst_height: u32,
  needs_fill: bool,
}

/// Calculate scaled dimensions based on scale mode
fn calculate_scaled_dimensions(
  buffer_width: u32,
  buffer_height: u32,
  window_width: u32,
  window_height: u32,
  auto_scale: bool,
  scale_mode: &ScaleMode,
) -> ScaledDimensions {
  if !auto_scale || (window_width == buffer_width && window_height == buffer_height) {
    return ScaledDimensions {
      src_x: 0,
      src_y: 0,
      src_width: buffer_width,
      src_height: buffer_height,
      dst_x: 0,
      dst_y: 0,
      dst_width: window_width,
      dst_height: window_height,
      needs_fill: false,
    };
  }

  match scale_mode {
    ScaleMode::Stretch => ScaledDimensions {
      src_x: 0,
      src_y: 0,
      src_width: buffer_width,
      src_height: buffer_height,
      dst_x: 0,
      dst_y: 0,
      dst_width: window_width,
      dst_height: window_height,
      needs_fill: false,
    },
    ScaleMode::Fit => {
      let buffer_aspect = buffer_width as f32 / buffer_height as f32;
      let window_aspect = window_width as f32 / window_height as f32;

      let (dst_width, dst_height, dst_x, dst_y) = if buffer_aspect > window_aspect {
        let scaled_height = (window_width as f32 / buffer_aspect) as u32;
        let y_offset = (window_height - scaled_height) / 2;
        (window_width, scaled_height, 0, y_offset)
      } else {
        let scaled_width = (window_height as f32 * buffer_aspect) as u32;
        let x_offset = (window_width - scaled_width) / 2;
        (scaled_width, window_height, x_offset, 0)
      };

      ScaledDimensions {
        src_x: 0,
        src_y: 0,
        src_width: buffer_width,
        src_height: buffer_height,
        dst_x,
        dst_y,
        dst_width,
        dst_height,
        needs_fill: true,
      }
    }
    ScaleMode::Fill => {
      let buffer_aspect = buffer_width as f32 / buffer_height as f32;
      let window_aspect = window_width as f32 / window_height as f32;

      let (src_x, src_y, src_width, src_height) = if buffer_aspect > window_aspect {
        let cropped_width = (buffer_height as f32 * window_aspect) as u32;
        let x_offset = (buffer_width - cropped_width) / 2;
        (x_offset, 0, cropped_width, buffer_height)
      } else {
        let cropped_height = (buffer_width as f32 / window_aspect) as u32;
        let y_offset = (buffer_height - cropped_height) / 2;
        (0, y_offset, buffer_width, cropped_height)
      };

      ScaledDimensions {
        src_x,
        src_y,
        src_width,
        src_height,
        dst_x: 0,
        dst_y: 0,
        dst_width: window_width,
        dst_height: window_height,
        needs_fill: false,
      }
    }
    ScaleMode::Integer => {
      let scale_x = window_width / buffer_width;
      let scale_y = window_height / buffer_height;
      let scale = scale_x.min(scale_y).max(1);

      let dst_width = buffer_width * scale;
      let dst_height = buffer_height * scale;
      let dst_x = (window_width - dst_width) / 2;
      let dst_y = (window_height - dst_height) / 2;

      ScaledDimensions {
        src_x: 0,
        src_y: 0,
        src_width: buffer_width,
        src_height: buffer_height,
        dst_x,
        dst_y,
        dst_width,
        dst_height,
        needs_fill: true,
      }
    }
    ScaleMode::None => {
      let dst_width = buffer_width.min(window_width);
      let dst_height = buffer_height.min(window_height);
      let dst_x = (window_width.saturating_sub(buffer_width)) / 2;
      let dst_y = (window_height.saturating_sub(buffer_height)) / 2;

      ScaledDimensions {
        src_x: 0,
        src_y: 0,
        src_width: dst_width,
        src_height: dst_height,
        dst_x,
        dst_y,
        dst_width,
        dst_height,
        needs_fill: true,
      }
    }
  }
}

/// Fill the background with a solid color
fn fill_background(
  surface_buffer: &mut softbuffer::Buffer<'_, &tao::window::Window, &tao::window::Window>,
  window_width: u32,
  window_height: u32,
  color: [u8; 4],
) {
  let argb = ((color[3] as u32) << 24)
    | ((color[0] as u32) << 16)
    | ((color[1] as u32) << 8)
    | (color[2] as u32);

  let total_pixels = (window_width * window_height) as usize;
  for i in 0..total_pixels.min(surface_buffer.len()) {
    surface_buffer[i] = argb;
  }
}

/// Scale and copy pixels from source buffer to surface buffer
fn scale_and_copy_pixels(
  surface_buffer: &mut softbuffer::Buffer<'_, &tao::window::Window, &tao::window::Window>,
  rgba_buffer: &[u8],
  buffer_width: u32,
  buffer_height: u32,
  window_width: u32,
  scaled: &ScaledDimensions,
) {
  // Convert background to ARGB format (softbuffer uses 0xAARRGGBB)

  // For each pixel in the destination region
  for dst_y in 0..scaled.dst_height {
    for dst_x in 0..scaled.dst_width {
      // Calculate source coordinates with scaling
      let src_x =
        scaled.src_x + ((dst_x as f32 / scaled.dst_width as f32) * scaled.src_width as f32) as u32;
      let src_y = scaled.src_y
        + ((dst_y as f32 / scaled.dst_height as f32) * scaled.src_height as f32) as u32;

      // Clamp to source bounds
      let src_x = src_x.min(buffer_width - 1);
      let src_y = src_y.min(buffer_height - 1);

      // Calculate indices
      let src_idx = ((src_y * buffer_width + src_x) * 4) as usize;
      let dst_idx = ((scaled.dst_y + dst_y) * window_width + (scaled.dst_x + dst_x)) as usize;

      if src_idx + 3 < rgba_buffer.len() && dst_idx < surface_buffer.len() {
        let r = rgba_buffer[src_idx] as u32;
        let g = rgba_buffer[src_idx + 1] as u32;
        let b = rgba_buffer[src_idx + 2] as u32;
        let a = rgba_buffer[src_idx + 3] as u32;
        // ARGB format
        surface_buffer[dst_idx] = (a << 24) | (r << 16) | (g << 8) | b;
      }
    }
  }
}
