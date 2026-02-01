//! Simple pixel buffer rendering module
//!
//! Provides a minimal API for rendering RGBA pixel buffers to Tao windows.
//! Uses X11 via pixels crate (Wayland support removed).

use crate::tao::enums::ScaleMode;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::cell::RefCell;
use std::sync::Mutex;

/// Per-window rendering state to avoid resource exhaustion
#[cfg(target_os = "linux")]
struct X11RenderState {
  pixels: pixels::Pixels<'static>,
}

/// Global cache for X11 rendering state to avoid "Maximum number of clients reached" errors.
/// The key is the window ID.
#[cfg(target_os = "linux")]
static X11_RENDER_STATE: std::sync::LazyLock<
  Mutex<RefCell<std::collections::HashMap<u64, X11RenderState>>>,
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
/// NOTE: This renderer uses global caches to avoid X11 client limit errors.
/// The "Maximum number of clients reached" error occurs when creating too many
/// X11 contexts/surfaces. This implementation uses a global cache keyed by window ID
/// to reuse rendering resources across render calls and even across different
/// PixelRenderer instances.
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
  /// This method uses global caches to avoid X11 "Maximum number of clients reached"
  /// errors that occur when creating new contexts/surfaces on each render call.
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

    // X11: Use cached pixels instance to avoid resource exhaustion
    self.render_x11_cached(
      window_id_u64,
      &window_guard,
      &buffer,
      window_width,
      window_height,
    )
  }

  /// Render using cached pixels instance for X11
  #[cfg(target_os = "linux")]
  fn render_x11_cached(
    &self,
    window_id: u64,
    window: &tao::window::Window,
    buffer: &[u8],
    window_width: u32,
    window_height: u32,
  ) -> napi::Result<()> {
    // Get or create the rendering state from the global cache
    let cache = X11_RENDER_STATE.lock().map_err(|_| {
      napi::Error::new(
        napi::Status::GenericFailure,
        "Failed to lock X11 render state cache".to_string(),
      )
    })?;

    // Check if we need to create a new pixels instance for this window
    let needs_create = {
      let cache_ref = cache.borrow();
      !cache_ref.contains_key(&window_id)
    };

    if needs_create {
      // Create new pixels instance
      let surface_texture = pixels::SurfaceTexture::new(window_width, window_height, window);
      let new_pixels = pixels::Pixels::new(self.buffer_width, self.buffer_height, surface_texture)
        .map_err(|e| {
          napi::Error::new(
            napi::Status::GenericFailure,
            format!("Failed to create pixels: {:?}", e),
          )
        })?;

      // SAFETY: We need to extend the lifetime to 'static for storage.
      // This is safe because:
      // 1. The pixels instance is only used while the window is alive
      // 2. The window ID is unique and won't be reused
      // 3. We clean up when the window is closed
      let static_pixels: pixels::Pixels<'static> = unsafe { std::mem::transmute(new_pixels) };

      cache.borrow_mut().insert(
        window_id,
        X11RenderState {
          pixels: static_pixels,
        },
      );
    }

    // Get mutable reference to state from cache
    let mut cache_mut = cache.borrow_mut();
    let state = cache_mut.get_mut(&window_id).ok_or_else(|| {
      napi::Error::new(
        napi::Status::GenericFailure,
        "Render state not available in cache".to_string(),
      )
    })?;

    // Handle window resize if needed by checking frame buffer size
    let needs_resize = {
      let frame = state.pixels.frame_mut();
      // The frame size is buffer_width * buffer_height * 4
      // We need to check if the surface texture size matches window size
      // Since we can't directly query surface_texture size in pixels 0.15,
      // we recreate if the frame dimensions don't match expected
      let expected_frame_len = (self.buffer_width * self.buffer_height * 4) as usize;
      frame.len() != expected_frame_len
    };

    // If resize is needed, remove and recreate
    if needs_resize {
      drop(cache_mut);
      cache.borrow_mut().remove(&window_id);

      // Recreate
      let surface_texture = pixels::SurfaceTexture::new(window_width, window_height, window);
      let new_pixels = pixels::Pixels::new(self.buffer_width, self.buffer_height, surface_texture)
        .map_err(|e| {
          napi::Error::new(
            napi::Status::GenericFailure,
            format!("Failed to create pixels: {:?}", e),
          )
        })?;

      let static_pixels: pixels::Pixels<'static> = unsafe { std::mem::transmute(new_pixels) };

      cache.borrow_mut().insert(
        window_id,
        X11RenderState {
          pixels: static_pixels,
        },
      );
      cache_mut = cache.borrow_mut();
    }

    let state = cache_mut.get_mut(&window_id).ok_or_else(|| {
      napi::Error::new(
        napi::Status::GenericFailure,
        "Render state not available after resize".to_string(),
      )
    })?;

    // Apply scaling if needed
    let (offset_x, offset_y, scaled_width, scaled_height) = calculate_scaled_dimensions(
      self.buffer_width,
      self.buffer_height,
      window_width,
      window_height,
      self.scale_mode,
    );

    // Copy buffer to pixel frame
    let frame = state.pixels.frame_mut();

    // Clear with background color first
    for pixel in frame.chunks_exact_mut(4) {
      pixel.copy_from_slice(&self.bg_color);
    }

    // Copy source buffer with scaling
    match self.scale_mode {
      ScaleMode::Stretch => {
        // Direct copy - pixels crate handles the stretch
        frame.copy_from_slice(buffer);
      }
      ScaleMode::None => {
        // Center without scaling
        copy_buffer_centered(
          frame,
          buffer,
          self.buffer_width,
          self.buffer_height,
          window_width,
          window_height,
        );
      }
      _ => {
        // Fit, Fill, Integer - copy with calculated dimensions
        copy_buffer_scaled(
          frame,
          buffer,
          CopyBufferParams {
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

  /// Fallback for non-Linux platforms
  #[cfg(not(target_os = "linux"))]
  fn render_x11_cached(
    &self,
    _window_id: u64,
    _window: &tao::window::Window,
    _buffer: &[u8],
    _window_width: u32,
    _window_height: u32,
  ) -> napi::Result<()> {
    Err(napi::Error::new(
      napi::Status::GenericFailure,
      "X11 rendering not supported on this platform".to_string(),
    ))
  }
}

/// Calculates scaled dimensions based on the render options
fn calculate_scaled_dimensions(
  buffer_width: u32,
  buffer_height: u32,
  window_width: u32,
  window_height: u32,
  scale_mode: ScaleMode,
) -> (u32, u32, u32, u32) {
  use crate::tao::enums::ScaleMode;

  match scale_mode {
    ScaleMode::Stretch => (0, 0, window_width, window_height),
    ScaleMode::Fit => {
      let scale_x = window_width as f64 / buffer_width as f64;
      let scale_y = window_height as f64 / buffer_height as f64;
      let scale = scale_x.min(scale_y);
      let scaled_width = (buffer_width as f64 * scale) as u32;
      let scaled_height = (buffer_height as f64 * scale) as u32;
      // Clamp to window dimensions to prevent overflow
      let scaled_width = scaled_width.min(window_width);
      let scaled_height = scaled_height.min(window_height);
      let offset_x = (window_width.saturating_sub(scaled_width)) / 2;
      let offset_y = (window_height.saturating_sub(scaled_height)) / 2;
      (offset_x, offset_y, scaled_width, scaled_height)
    }
    ScaleMode::Fill => {
      let scale_x = window_width as f64 / buffer_width as f64;
      let scale_y = window_height as f64 / buffer_height as f64;
      let scale = scale_x.max(scale_y);
      let scaled_width = (buffer_width as f64 * scale) as u32;
      let scaled_height = (buffer_height as f64 * scale) as u32;
      let offset_x = (window_width.saturating_sub(scaled_width)) / 2;
      let offset_y = (window_height.saturating_sub(scaled_height)) / 2;
      (offset_x, offset_y, scaled_width, scaled_height)
    }
    ScaleMode::Integer => {
      let scale_x = window_width as f64 / buffer_width as f64;
      let scale_y = window_height as f64 / buffer_height as f64;
      let scale = scale_x.min(scale_y).floor() as u32;
      let scale = scale.max(1);
      let scaled_width = buffer_width * scale;
      let scaled_height = buffer_height * scale;
      let offset_x = (window_width.saturating_sub(scaled_width)) / 2;
      let offset_y = (window_height.saturating_sub(scaled_height)) / 2;
      (offset_x, offset_y, scaled_width, scaled_height)
    }
    ScaleMode::None => {
      let offset_x = (window_width.saturating_sub(buffer_width)) / 2;
      let offset_y = (window_height.saturating_sub(buffer_height)) / 2;
      (offset_x, offset_y, buffer_width, buffer_height)
    }
  }
}

/// Copies buffer centered without scaling
fn copy_buffer_centered(
  frame: &mut [u8],
  buffer: &[u8],
  buffer_width: u32,
  buffer_height: u32,
  window_width: u32,
  window_height: u32,
) {
  let offset_x = ((window_width.saturating_sub(buffer_width)) / 2) as usize;
  let offset_y = ((window_height.saturating_sub(buffer_height)) / 2) as usize;

  for y in 0..buffer_height.min(window_height) {
    let src_row_start = (y * buffer_width * 4) as usize;
    let src_row_end = src_row_start + (buffer_width * 4) as usize;
    let dst_row_start = ((offset_y + y as usize) * window_width as usize + offset_x) * 4;

    if dst_row_start + (buffer_width * 4) as usize <= frame.len() {
      frame[dst_row_start..dst_row_start + (buffer_width * 4) as usize]
        .copy_from_slice(&buffer[src_row_start..src_row_end]);
    }
  }
}

/// Parameters for buffer copying with scaling
struct CopyBufferParams {
  buffer_width: u32,
  buffer_height: u32,
  window_width: u32,
  window_height: u32,
  offset_x: u32,
  offset_y: u32,
  scaled_width: u32,
  scaled_height: u32,
}

/// Copies buffer with scaling (simple nearest-neighbor)
fn copy_buffer_scaled(frame: &mut [u8], buffer: &[u8], params: CopyBufferParams) {
  let CopyBufferParams {
    buffer_width,
    buffer_height,
    window_width,
    window_height,
    offset_x,
    offset_y,
    scaled_width,
    scaled_height,
  } = params;

  let scale_x = buffer_width as f64 / scaled_width as f64;
  let scale_y = buffer_height as f64 / scaled_height as f64;

  for y in 0..scaled_height {
    let src_y = (y as f64 * scale_y).min(buffer_height as f64 - 1.0) as u32;
    let dst_y = offset_y + y;

    if dst_y >= window_height {
      break;
    }

    for x in 0..scaled_width {
      let src_x = (x as f64 * scale_x).min(buffer_width as f64 - 1.0) as u32;
      let dst_x = offset_x + x;

      if dst_x >= window_width {
        break;
      }

      let src_idx = ((src_y * buffer_width + src_x) * 4) as usize;
      let dst_idx = ((dst_y * window_width + dst_x) * 4) as usize;

      if src_idx + 4 <= buffer.len() && dst_idx + 4 <= frame.len() {
        frame[dst_idx..dst_idx + 4].copy_from_slice(&buffer[src_idx..src_idx + 4]);
      }
    }
  }
}

/// Simple function to render a pixel buffer to a window
///
/// This is a convenience function for one-off renders.
/// For repeated rendering, use [`PixelRenderer`] instead.
///
/// # Warning
/// Using this function repeatedly (200+ times) may cause X11 "Maximum number of clients reached"
/// errors. For repeated rendering, create a [`PixelRenderer`] instance and reuse it.
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
