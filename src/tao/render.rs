//! Simple pixel buffer rendering module
//!
//! Provides a minimal API for rendering RGBA pixel buffers to Tao windows.

use crate::tao::enums::ScaleMode;
use napi::bindgen_prelude::*;
use napi_derive::napi;

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

    // Create pixels surface and renderer
    let surface_texture = pixels::SurfaceTexture::new(window_width, window_height, &*window_guard);

    let mut pixels = pixels::Pixels::new(self.buffer_width, self.buffer_height, surface_texture)
      .map_err(|e| {
        napi::Error::new(
          napi::Status::GenericFailure,
          format!("Failed to create pixels: {:?}", e),
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
    let frame = pixels.frame_mut();

    // Clear with background color first
    for pixel in frame.chunks_exact_mut(4) {
      pixel.copy_from_slice(&self.bg_color);
    }

    // Copy source buffer with scaling
    match self.scale_mode {
      ScaleMode::Stretch => {
        // Direct copy - pixels crate handles the stretch
        frame.copy_from_slice(&buffer);
      }
      ScaleMode::None => {
        // Center without scaling
        copy_buffer_centered(
          frame,
          &buffer,
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
          &buffer,
          self.buffer_width,
          self.buffer_height,
          window_width,
          window_height,
          offset_x,
          offset_y,
          scaled_width,
          scaled_height,
        );
      }
    }

    // Render
    pixels.render().map_err(|e| {
      napi::Error::new(
        napi::Status::GenericFailure,
        format!("Failed to render: {:?}", e),
      )
    })?;

    Ok(())
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
      let offset_x = (window_width - scaled_width) / 2;
      let offset_y = (window_height - scaled_height) / 2;
      (offset_x, offset_y, scaled_width, scaled_height)
    }
    ScaleMode::Fill => {
      let scale_x = window_width as f64 / buffer_width as f64;
      let scale_y = window_height as f64 / buffer_height as f64;
      let scale = scale_x.max(scale_y);
      let scaled_width = (buffer_width as f64 * scale) as u32;
      let scaled_height = (buffer_height as f64 * scale) as u32;
      let offset_x = (window_width - scaled_width) / 2;
      let offset_y = (window_height - scaled_height) / 2;
      (offset_x, offset_y, scaled_width, scaled_height)
    }
    ScaleMode::Integer => {
      let scale_x = window_width as f64 / buffer_width as f64;
      let scale_y = window_height as f64 / buffer_height as f64;
      let scale = scale_x.min(scale_y).floor() as u32;
      let scale = scale.max(1);
      let scaled_width = buffer_width * scale;
      let scaled_height = buffer_height * scale;
      let offset_x = (window_width - scaled_width) / 2;
      let offset_y = (window_height - scaled_height) / 2;
      (offset_x, offset_y, scaled_width, scaled_height)
    }
    ScaleMode::None => {
      let offset_x = (window_width - buffer_width) / 2;
      let offset_y = (window_height - buffer_height) / 2;
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

/// Copies buffer with scaling (simple nearest-neighbor)
fn copy_buffer_scaled(
  frame: &mut [u8],
  buffer: &[u8],
  buffer_width: u32,
  buffer_height: u32,
  window_width: u32,
  window_height: u32,
  offset_x: u32,
  offset_y: u32,
  scaled_width: u32,
  scaled_height: u32,
) {
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
