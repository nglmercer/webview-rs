//! Pixel renderer module
//!
//! Provides a minimal API for rendering RGBA pixel buffers to Tao windows.
//! Uses the pixels crate which supports multiple backends (X11, DXGI, Cocoa).

use crate::tao::enums::ScaleMode;
use crate::tao::render::scaling::calculate_scaled_dimensions;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::cell::RefCell;
use std::sync::Mutex;

// Debug logging macro - set to false to disable debug output
const DEBUG_ENABLED: bool = false;

#[allow(unused_macros)]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        if DEBUG_ENABLED {
            eprintln!("[PixelRenderer] {}", format!($($arg)*));
        }
    };
}

/// Per-window rendering state to avoid resource exhaustion
struct RenderState {
  pixels: pixels::Pixels<'static>,
  last_window_width: u32,
  last_window_height: u32,
}

/// Global cache for rendering state to avoid resource exhaustion errors.
/// The key is the window ID. Works on all platforms (X11, DXGI, Cocoa).
static RENDER_STATE: std::sync::LazyLock<
  Mutex<RefCell<std::collections::HashMap<u64, RenderState>>>,
> = std::sync::LazyLock::new(|| Mutex::new(RefCell::new(std::collections::HashMap::new())));

/// Render options for pixel buffer display
#[napi(object)]
#[derive(Debug, Clone)]
pub struct RenderOptions {
  /// Width of the source buffer in pixels
  pub buffer_width: u32,
  /// Height of the source buffer in pixels
  pub buffer_height: u32,
  /// Scaling mode (default: Fit)
  pub scale_mode: Option<ScaleMode>,
  /// Background color for letterboxing [R, G, B, A] (default: [0, 0, 0, 255])
  pub background_color: Option<Vec<u8>>,
}

impl Default for RenderOptions {
  fn default() -> Self {
    Self {
      buffer_width: 800,
      buffer_height: 600,
      scale_mode: Some(ScaleMode::Fit),
      background_color: Some(vec![0, 0, 0, 255]),
    }
  }
}

/// Simple pixel renderer for Tao windows
///
/// NOTE: This renderer uses a global cache to avoid resource exhaustion errors
/// that occur when creating too many contexts/surfaces on each render call.
/// Resources are cached per-window and reused across all PixelRenderer instances.
#[napi]
pub struct PixelRenderer {
  buffer_width: u32,
  buffer_height: u32,
  scale_mode: ScaleMode,
  bg_color: [u8; 4],
}

#[napi]
impl PixelRenderer {
  /// Creates a new pixel renderer with the given buffer dimensions
  #[napi(constructor)]
  pub fn new(buffer_width: u32, buffer_height: u32) -> Self {
    Self {
      buffer_width,
      buffer_height,
      scale_mode: ScaleMode::Fit,
      bg_color: [0, 0, 0, 255],
    }
  }

  /// Creates a new pixel renderer with options
  #[napi(factory)]
  pub fn with_options(options: RenderOptions) -> Self {
    let bg_color = options
      .background_color
      .as_ref()
      .and_then(|c| {
        if c.len() >= 4 {
          Some([c[0], c[1], c[2], c[3]])
        } else {
          None
        }
      })
      .unwrap_or([0, 0, 0, 255]);

    Self {
      buffer_width: options.buffer_width,
      buffer_height: options.buffer_height,
      scale_mode: options.scale_mode.unwrap_or(ScaleMode::Fit),
      bg_color,
    }
  }

  /// Sets the scaling mode
  #[napi]
  pub fn set_scale_mode(&mut self, mode: ScaleMode) {
    self.scale_mode = mode;
  }

  /// Sets the background color
  #[napi]
  pub fn set_background_color(&mut self, r: u8, g: u8, b: u8, a: u8) {
    self.bg_color = [r, g, b, a];
  }

  /// Renders a pixel buffer to the given window
  ///
  /// # Arguments
  /// * `window` - The Tao window to render to
  /// * `buffer` - RGBA pixel buffer (must be buffer_width * buffer_height * 4 bytes)
  ///
  /// # Performance Note
  /// This method uses a global cache to avoid resource exhaustion errors
  /// that occur when creating new contexts/surfaces on each render call.
  /// Resources are cached per-window and reused across all PixelRenderer instances.
  #[napi]
  pub fn render(&self, window: &crate::tao::structs::Window, buffer: Buffer) -> napi::Result<()> {
    let window_arc = window.inner.as_ref().ok_or_else(|| {
      napi::Error::new(
        napi::Status::GenericFailure,
        "Window not initialized".to_string(),
      )
    })?;

    let window_guard = window_arc.lock().map_err(|_| {
      napi::Error::new(
        napi::Status::GenericFailure,
        "Failed to lock window".to_string(),
      )
    })?;

    // Get the window ID for caching
    let window_id = window_guard.id();
    let window_id_u64 = unsafe {
      let mut id_val: u64 = 0;
      std::ptr::copy_nonoverlapping(
        &window_id as *const _ as *const u8,
        &mut id_val as *mut _ as *mut u8,
        std::mem::size_of_val(&window_id).min(8),
      );
      id_val
    };

    let window_size = window_guard.inner_size();
    let window_width = window_size.width;
    let window_height = window_size.height;

    // Validate buffer size
    let expected_len = (self.buffer_width * self.buffer_height * 4) as usize;
    if buffer.len() != expected_len {
      return Err(napi::Error::new(
        napi::Status::GenericFailure,
        format!(
          "Buffer size mismatch: got {} bytes, expected {} bytes for {}x{}",
          buffer.len(),
          expected_len,
          self.buffer_width,
          self.buffer_height
        ),
      ));
    }

    // Render using cached pixels instance
    self.render_cached(
      window_id_u64,
      &window_guard,
      &buffer,
      window_width,
      window_height,
    )
  }

  /// Render using cached pixels instance (platform-agnostic)
  fn render_cached(
    &self,
    window_id: u64,
    window: &tao::window::Window,
    buffer: &[u8],
    window_width: u32,
    window_height: u32,
  ) -> napi::Result<()> {
    // Get or create the rendering state from the global cache using entry API
    let cache = RENDER_STATE.lock().map_err(|_| {
      napi::Error::new(
        napi::Status::GenericFailure,
        "Failed to lock render state cache".to_string(),
      )
    })?;

    // Use entry API for single lookup - more efficient than contains_key + get_mut
    let mut cache_ref = cache.borrow_mut();
    let state = cache_ref.entry(window_id).or_insert_with(|| {
      // Create new pixels instance with window dimensions
      let surface_texture = pixels::SurfaceTexture::new(window_width, window_height, window);
      let new_pixels = pixels::Pixels::new(window_width, window_height, surface_texture)
        .expect("Failed to create pixels instance");

      // SAFETY: Extending lifetime to 'static is safe because:
      // 1. The pixels instance is only used while the window is alive
      // 2. The window ID is unique and won't be reused
      // 3. We clean up when the window is closed
      let static_pixels: pixels::Pixels<'static> = unsafe { std::mem::transmute(new_pixels) };

      RenderState {
        pixels: static_pixels,
        last_window_width: window_width,
        last_window_height: window_height,
      }
    });

    // Handle window resize if needed
    let needs_resize =
      state.last_window_width != window_width || state.last_window_height != window_height;

    if needs_resize {
      debug_log!(
        "  window resized: {}x{} -> {}x{}",
        state.last_window_width,
        state.last_window_height,
        window_width,
        window_height
      );

      // Try to resize the surface texture to match the new window size
      if let Err(e) = state.pixels.resize_surface(window_width, window_height) {
        debug_log!(
          "  resize_surface failed: {:?}, recreating pixels instance",
          e
        );
        // If resize fails, fall back to recreating
        // Drop the current borrow of the hashmap
        drop(cache_ref);

        // Get mutable access to cache and recreate
        let mut cache_mut = cache.borrow_mut();
        cache_mut.remove(&window_id);

        let surface_texture = pixels::SurfaceTexture::new(window_width, window_height, window);
        let new_pixels = pixels::Pixels::new(window_width, window_height, surface_texture)
          .map_err(|e| {
            napi::Error::new(
              napi::Status::GenericFailure,
              format!("Failed to create pixels: {:?}", e),
            )
          })?;

        let static_pixels: pixels::Pixels<'static> = unsafe { std::mem::transmute(new_pixels) };

        cache_mut.insert(
          window_id,
          RenderState {
            pixels: static_pixels,
            last_window_width: window_width,
            last_window_height: window_height,
          },
        );

        // Get the newly inserted state
        let state = cache_mut.get_mut(&window_id).ok_or_else(|| {
          napi::Error::new(
            napi::Status::GenericFailure,
            "Render state not available after recreation".to_string(),
          )
        })?;

        // Continue with rendering using the new state
        return self.render_with_state(state, buffer, window_width, window_height);
      } else {
        // Also resize the pixel buffer to match window dimensions
        if let Err(e) = state.pixels.resize_buffer(window_width, window_height) {
          debug_log!("  resize_buffer failed: {:?}", e);
        }

        // Update cached window size
        state.last_window_width = window_width;
        state.last_window_height = window_height;
        debug_log!(
          "  resized surface and buffer to {}x{}",
          window_width,
          window_height
        );
      }
    }

    self.render_with_state(state, buffer, window_width, window_height)
  }

  /// Render using an already acquired state
  fn render_with_state(
    &self,
    state: &mut RenderState,
    buffer: &[u8],
    window_width: u32,
    window_height: u32,
  ) -> napi::Result<()> {
    // Apply scaling if needed
    let (offset_x, offset_y, scaled_width, scaled_height) = calculate_scaled_dimensions(
      self.buffer_width,
      self.buffer_height,
      window_width,
      window_height,
      self.scale_mode,
    );

    debug_log!(
      "render_with_state: buffer={}x{}, window={}x{}, scale_mode={:?}",
      self.buffer_width,
      self.buffer_height,
      window_width,
      window_height,
      self.scale_mode
    );
    debug_log!(
      "  calculated: offset=({}, {}), scaled={}x{}",
      offset_x,
      offset_y,
      scaled_width,
      scaled_height
    );

    // Copy buffer to pixel frame
    let frame = state.pixels.frame_mut();
    debug_log!(
      "  frame.len()={}, expected={}",
      frame.len(),
      window_width * window_height * 4
    );

    // Clear with background color first
    for pixel in frame.chunks_exact_mut(4) {
      pixel.copy_from_slice(&self.bg_color);
    }

    // Copy source buffer with scaling
    // The frame buffer is sized to window_width x window_height
    // We need to scale the source buffer to fit properly
    match self.scale_mode {
      ScaleMode::Stretch => {
        // Stretch mode: scale entire buffer to fill window
        scale_buffer_nearest_neighbor(
          frame,
          buffer,
          self.buffer_width,
          self.buffer_height,
          window_width,
          window_height,
        );
      }
      ScaleMode::None => {
        // Center without scaling, crop if buffer is larger than window
        copy_buffer_centered_crop(
          frame,
          buffer,
          self.buffer_width,
          self.buffer_height,
          window_width,
          window_height,
        );
      }
      ScaleMode::Fill => {
        // Fill mode: scale buffer maintaining aspect ratio to fill window
        scale_buffer_fill(
          frame,
          buffer,
          self.buffer_width,
          self.buffer_height,
          window_width,
          window_height,
        );
      }
      _ => {
        // Fit, Integer - scale buffer maintaining aspect ratio to fit within window
        scale_buffer_fit(
          frame,
          buffer,
          ScaleBufferFitParams {
            buffer_width: self.buffer_width,
            buffer_height: self.buffer_height,
            window_width,
            window_height,
            offset_x,
            offset_y,
            scaled_width,
            scaled_height,
          },
        );
      }
    }

    // Render
    state.pixels.render().map_err(|e| {
      napi::Error::new(
        napi::Status::GenericFailure,
        format!("Failed to render: {:?}", e),
      )
    })?;

    Ok(())
  }
}

/// Simple function to render a pixel buffer to a window
///
/// This is a convenience function for one-off renders.
/// For repeated rendering, use [`PixelRenderer`] instead.
///
/// # Warning
/// Using this function repeatedly (200+ times) may cause resource exhaustion errors.
/// For repeated rendering, create a [`PixelRenderer`] instance and reuse it.
#[napi]
pub fn render_pixels(
  window: &crate::tao::structs::Window,
  buffer: Buffer,
  buffer_width: u32,
  buffer_height: u32,
) -> napi::Result<()> {
  let renderer = PixelRenderer::new(buffer_width, buffer_height);
  renderer.render(window, buffer)
}

pub mod buffer_ops;
pub mod scaling;

/// Scales buffer to fill the entire window using nearest neighbor
fn scale_buffer_nearest_neighbor(
  frame: &mut [u8],
  buffer: &[u8],
  buffer_width: u32,
  buffer_height: u32,
  window_width: u32,
  window_height: u32,
) {
  for y in 0..window_height {
    for x in 0..window_width {
      let src_x = (x as f32 * buffer_width as f32 / window_width as f32)
        .min(buffer_width as f32 - 1.0) as u32;
      let src_y = (y as f32 * buffer_height as f32 / window_height as f32)
        .min(buffer_height as f32 - 1.0) as u32;

      let src_idx = ((src_y * buffer_width + src_x) * 4) as usize;
      let dst_idx = ((y * window_width + x) * 4) as usize;

      if src_idx + 4 <= buffer.len() && dst_idx + 4 <= frame.len() {
        frame[dst_idx..dst_idx + 4].copy_from_slice(&buffer[src_idx..src_idx + 4]);
      }
    }
  }
}

/// Centers buffer without scaling, cropping if necessary
fn copy_buffer_centered_crop(
  frame: &mut [u8],
  buffer: &[u8],
  buffer_width: u32,
  buffer_height: u32,
  window_width: u32,
  window_height: u32,
) {
  let crop_x = buffer_width.saturating_sub(window_width) / 2;
  let crop_y = buffer_height.saturating_sub(window_height) / 2;
  let copy_width = buffer_width.min(window_width);
  let copy_height = buffer_height.min(window_height);
  let start_x = (window_width.saturating_sub(buffer_width)) / 2;
  let start_y = (window_height.saturating_sub(buffer_height)) / 2;

  for y in 0..copy_height {
    for x in 0..copy_width {
      let src_x = crop_x + x;
      let src_y = crop_y + y;
      let dst_x = start_x + x;
      let dst_y = start_y + y;

      let src_idx = ((src_y * buffer_width + src_x) * 4) as usize;
      let dst_idx = ((dst_y * window_width + dst_x) * 4) as usize;

      if src_idx + 4 <= buffer.len() && dst_idx + 4 <= frame.len() {
        frame[dst_idx..dst_idx + 4].copy_from_slice(&buffer[src_idx..src_idx + 4]);
      }
    }
  }
}

/// Scales buffer to fill window, maintaining aspect ratio by cropping
fn scale_buffer_fill(
  frame: &mut [u8],
  buffer: &[u8],
  buffer_width: u32,
  buffer_height: u32,
  window_width: u32,
  window_height: u32,
) {
  let buffer_aspect = buffer_width as f32 / buffer_height as f32;
  let window_aspect = window_width as f32 / window_height as f32;

  let (crop_x, crop_y, crop_width, crop_height) = if buffer_aspect > window_aspect {
    let new_width = (buffer_height as f32 * window_aspect) as u32;
    ((buffer_width - new_width) / 2, 0, new_width, buffer_height)
  } else {
    let new_height = (buffer_width as f32 / window_aspect) as u32;
    (
      0,
      (buffer_height - new_height) / 2,
      buffer_width,
      new_height,
    )
  };

  for y in 0..window_height {
    for x in 0..window_width {
      let src_x = crop_x
        + (x as f32 * crop_width as f32 / window_width as f32).min(crop_width as f32 - 1.0) as u32;
      let src_y = crop_y
        + (y as f32 * crop_height as f32 / window_height as f32).min(crop_height as f32 - 1.0)
          as u32;

      let src_idx = ((src_y * buffer_width + src_x) * 4) as usize;
      let dst_idx = ((y * window_width + x) * 4) as usize;

      if src_idx + 4 <= buffer.len() && dst_idx + 4 <= frame.len() {
        frame[dst_idx..dst_idx + 4].copy_from_slice(&buffer[src_idx..src_idx + 4]);
      }
    }
  }
}

/// Parameters for scaling buffer to fit window
struct ScaleBufferFitParams {
  buffer_width: u32,
  buffer_height: u32,
  window_width: u32,
  window_height: u32,
  offset_x: u32,
  offset_y: u32,
  scaled_width: u32,
  scaled_height: u32,
}

/// Scales buffer to fit window, maintaining aspect ratio with letterboxing
fn scale_buffer_fit(frame: &mut [u8], buffer: &[u8], params: ScaleBufferFitParams) {
  let ScaleBufferFitParams {
    buffer_width,
    buffer_height,
    window_width,
    window_height,
    offset_x,
    offset_y,
    scaled_width,
    scaled_height,
  } = params;

  // Frame is already cleared with background color

  for y in 0..scaled_height {
    for x in 0..scaled_width {
      let src_x = (x as f32 * buffer_width as f32 / scaled_width as f32)
        .min(buffer_width as f32 - 1.0) as u32;
      let src_y = (y as f32 * buffer_height as f32 / scaled_height as f32)
        .min(buffer_height as f32 - 1.0) as u32;

      let dst_x = offset_x + x;
      let dst_y = offset_y + y;

      if dst_x < window_width && dst_y < window_height {
        let src_idx = ((src_y * buffer_width + src_x) * 4) as usize;
        let dst_idx = ((dst_y * window_width + dst_x) * 4) as usize;

        if src_idx + 4 <= buffer.len() && dst_idx + 4 <= frame.len() {
          frame[dst_idx..dst_idx + 4].copy_from_slice(&buffer[src_idx..src_idx + 4]);
        }
      }
    }
  }
}
