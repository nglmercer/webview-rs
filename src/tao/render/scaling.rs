//! Scaling and dimension calculations for pixel buffer rendering
//!
//! This module provides functions for calculating scaled dimensions
//! and offsets based on different scaling modes.

use crate::tao::enums::ScaleMode;

/// Calculates scaled dimensions based on the render options
///
/// Returns a tuple of (offset_x, offset_y, scaled_width, scaled_height)
///
/// # Arguments
/// * `buffer_width` - Width of the source buffer in pixels
/// * `buffer_height` - Height of the source buffer in pixels
/// * `window_width` - Width of the target window in pixels
/// * `window_height` - Height of the target window in pixels
/// * `scale_mode` - The scaling mode to use
pub fn calculate_scaled_dimensions(
  buffer_width: u32,
  buffer_height: u32,
  window_width: u32,
  window_height: u32,
  scale_mode: ScaleMode,
) -> (u32, u32, u32, u32) {
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

#[cfg(test)]
mod tests {
  use super::*;

  // ============================================================================
  // ScaleMode::Fit Tests
  // ============================================================================

  #[test]
  fn test_fit_16_9_buffer_to_16_9_window() {
    // 1920x1080 buffer to 1920x1080 window - exact fit
    let (offset_x, offset_y, scaled_w, scaled_h) =
      calculate_scaled_dimensions(1920, 1080, 1920, 1080, ScaleMode::Fit);
    assert_eq!(offset_x, 0);
    assert_eq!(offset_y, 0);
    assert_eq!(scaled_w, 1920);
    assert_eq!(scaled_h, 1080);
  }

  #[test]
  fn test_fit_16_9_buffer_to_4_3_window() {
    // 1920x1080 (16:9) buffer to 800x600 (4:3) window
    // Should letterbox (black bars top/bottom)
    let (offset_x, offset_y, scaled_w, scaled_h) =
      calculate_scaled_dimensions(1920, 1080, 800, 600, ScaleMode::Fit);
    // Scale is limited by width: 800/1920 = 0.4167
    // Scaled height: 1080 * 0.4167 = 450
    assert_eq!(scaled_w, 800);
    assert_eq!(scaled_h, 450);
    assert_eq!(offset_x, 0);
    assert_eq!(offset_y, 75); // (600 - 450) / 2
  }

  #[test]
  fn test_fit_4_3_buffer_to_16_9_window() {
    // 800x600 (4:3) buffer to 1920x1080 (16:9) window
    // Should pillarbox (black bars left/right)
    let (offset_x, offset_y, scaled_w, scaled_h) =
      calculate_scaled_dimensions(800, 600, 1920, 1080, ScaleMode::Fit);
    // Scale is limited by height: 1080/600 = 1.8
    // Scaled width: 800 * 1.8 = 1440
    assert_eq!(scaled_w, 1440);
    assert_eq!(scaled_h, 1080);
    assert_eq!(offset_x, 240); // (1920 - 1440) / 2
    assert_eq!(offset_y, 0);
  }

  #[test]
  fn test_fit_buffer_larger_than_window() {
    // 3840x2160 buffer to 1920x1080 window
    let (offset_x, offset_y, scaled_w, scaled_h) =
      calculate_scaled_dimensions(3840, 2160, 1920, 1080, ScaleMode::Fit);
    assert_eq!(scaled_w, 1920);
    assert_eq!(scaled_h, 1080);
    assert_eq!(offset_x, 0);
    assert_eq!(offset_y, 0);
  }

  #[test]
  fn test_fit_buffer_smaller_than_window() {
    // 320x240 buffer to 1920x1080 window
    let (offset_x, offset_y, scaled_w, scaled_h) =
      calculate_scaled_dimensions(320, 240, 1920, 1080, ScaleMode::Fit);
    // Scale limited by height: 1080/240 = 4.5
    // Scaled width: 320 * 4.5 = 1440
    assert_eq!(scaled_w, 1440);
    assert_eq!(scaled_h, 1080);
    assert_eq!(offset_x, 240);
    assert_eq!(offset_y, 0);
  }

  #[test]
  fn test_fit_1_1_aspect_ratio() {
    // 512x512 buffer to 1024x768 window
    let (offset_x, offset_y, scaled_w, scaled_h) =
      calculate_scaled_dimensions(512, 512, 1024, 768, ScaleMode::Fit);
    // Scale limited by height: 768/512 = 1.5
    assert_eq!(scaled_w, 768);
    assert_eq!(scaled_h, 768);
    assert_eq!(offset_x, 128); // (1024 - 768) / 2
    assert_eq!(offset_y, 0);
  }

  #[test]
  fn test_fit_21_9_ultrawide() {
    // 2560x1080 (21:9) buffer to 1920x1080 (16:9) window
    let (offset_x, offset_y, scaled_w, scaled_h) =
      calculate_scaled_dimensions(2560, 1080, 1920, 1080, ScaleMode::Fit);
    // Scale limited by height: 1080/1080 = 1
    assert_eq!(scaled_w, 1920);
    assert_eq!(scaled_h, 810); // 1080 * (1920/2560)
    assert_eq!(offset_x, 0);
    assert_eq!(offset_y, 135); // (1080 - 810) / 2
  }

  // ============================================================================
  // ScaleMode::Fill Tests
  // ============================================================================

  #[test]
  fn test_fill_16_9_buffer_to_16_9_window() {
    // 1920x1080 buffer to 1920x1080 window - exact fit
    let (offset_x, offset_y, scaled_w, scaled_h) =
      calculate_scaled_dimensions(1920, 1080, 1920, 1080, ScaleMode::Fill);
    assert_eq!(offset_x, 0);
    assert_eq!(offset_y, 0);
    assert_eq!(scaled_w, 1920);
    assert_eq!(scaled_h, 1080);
  }

  #[test]
  fn test_fill_16_9_buffer_to_4_3_window() {
    // 1920x1080 (16:9) buffer to 800x600 (4:3) window
    // Should fill width, crop height
    let (offset_x, offset_y, scaled_w, scaled_h) =
      calculate_scaled_dimensions(1920, 1080, 800, 600, ScaleMode::Fill);
    // Scale is max of the two: max(800/1920, 600/1080) = max(0.4167, 0.5556) = 0.5556
    // 1920 * 0.5556 = 1066.67, truncated to 1066
    assert_eq!(scaled_w, 1066);
    assert_eq!(scaled_h, 600);
    // offset_x uses saturating_sub: 800.saturating_sub(1066) = 0, /2 = 0
    assert_eq!(offset_x, 0);
    assert_eq!(offset_y, 0);
  }

  #[test]
  fn test_fill_4_3_buffer_to_16_9_window() {
    // 800x600 (4:3) buffer to 1920x1080 (16:9) window
    // Should fill height, crop width
    let (offset_x, offset_y, scaled_w, scaled_h) =
      calculate_scaled_dimensions(800, 600, 1920, 1080, ScaleMode::Fill);
    // Scale is max: max(1920/800, 1080/600) = max(2.4, 1.8) = 2.4
    assert_eq!(scaled_w, 1920);
    assert_eq!(scaled_h, 1440); // 600 * 2.4
    assert_eq!(offset_x, 0);
    // offset_y uses saturating_sub: 1080.saturating_sub(1440) = 0
    assert_eq!(offset_y, 0);
  }

  // ============================================================================
  // ScaleMode::Integer Tests
  // ============================================================================

  #[test]
  fn test_integer_exact_double() {
    // 640x480 buffer to 1280x960 window - exactly 2x
    let (offset_x, offset_y, scaled_w, scaled_h) =
      calculate_scaled_dimensions(640, 480, 1280, 960, ScaleMode::Integer);
    assert_eq!(scaled_w, 1280);
    assert_eq!(scaled_h, 960);
    assert_eq!(offset_x, 0);
    assert_eq!(offset_y, 0);
  }

  #[test]
  fn test_integer_partial_scale() {
    // 320x240 buffer to 800x600 window
    // Scale x = 2.5, Scale y = 2.5, floor = 2
    let (offset_x, offset_y, scaled_w, scaled_h) =
      calculate_scaled_dimensions(320, 240, 800, 600, ScaleMode::Integer);
    assert_eq!(scaled_w, 640); // 320 * 2
    assert_eq!(scaled_h, 480); // 240 * 2
    assert_eq!(offset_x, 80); // (800 - 640) / 2
    assert_eq!(offset_y, 60); // (600 - 480) / 2
  }

  #[test]
  fn test_integer_minimum_scale_one() {
    // Very large buffer to small window - scale should be at least 1
    let (offset_x, offset_y, scaled_w, scaled_h) =
      calculate_scaled_dimensions(3840, 2160, 640, 480, ScaleMode::Integer);
    // Scale would be < 1, so it clamps to 1
    assert_eq!(scaled_w, 3840);
    assert_eq!(scaled_h, 2160);
    assert_eq!(offset_x, 0); // saturating_sub
    assert_eq!(offset_y, 0); // saturating_sub
  }

  #[test]
  fn test_integer_3x_scale() {
    // 256x224 buffer (SNES resolution) to 1024x768 window
    // Scale x = 4.0, Scale y = 3.43, floor = 3
    let (offset_x, offset_y, scaled_w, scaled_h) =
      calculate_scaled_dimensions(256, 224, 1024, 768, ScaleMode::Integer);
    assert_eq!(scaled_w, 768); // 256 * 3
    assert_eq!(scaled_h, 672); // 224 * 3
    assert_eq!(offset_x, 128); // (1024 - 768) / 2
    assert_eq!(offset_y, 48); // (768 - 672) / 2
  }

  // ============================================================================
  // ScaleMode::None Tests
  // ============================================================================

  #[test]
  fn test_none_exact_size() {
    // Same size - no offset
    let (offset_x, offset_y, scaled_w, scaled_h) =
      calculate_scaled_dimensions(800, 600, 800, 600, ScaleMode::None);
    assert_eq!(scaled_w, 800);
    assert_eq!(scaled_h, 600);
    assert_eq!(offset_x, 0);
    assert_eq!(offset_y, 0);
  }

  #[test]
  fn test_none_centered() {
    // Smaller buffer centered in larger window
    let (offset_x, offset_y, scaled_w, scaled_h) =
      calculate_scaled_dimensions(640, 480, 800, 600, ScaleMode::None);
    assert_eq!(scaled_w, 640);
    assert_eq!(scaled_h, 480);
    assert_eq!(offset_x, 80); // (800 - 640) / 2
    assert_eq!(offset_y, 60); // (600 - 480) / 2
  }

  #[test]
  fn test_none_buffer_larger_than_window() {
    // Larger buffer - should use saturating_sub
    let (offset_x, offset_y, scaled_w, scaled_h) =
      calculate_scaled_dimensions(1920, 1080, 800, 600, ScaleMode::None);
    assert_eq!(scaled_w, 1920);
    assert_eq!(scaled_h, 1080);
    assert_eq!(offset_x, 0); // saturating_sub
    assert_eq!(offset_y, 0); // saturating_sub
  }

  // ============================================================================
  // ScaleMode::Stretch Tests
  // ============================================================================

  #[test]
  fn test_stretch_exact() {
    let (offset_x, offset_y, scaled_w, scaled_h) =
      calculate_scaled_dimensions(800, 600, 800, 600, ScaleMode::Stretch);
    assert_eq!(scaled_w, 800);
    assert_eq!(scaled_h, 600);
    assert_eq!(offset_x, 0);
    assert_eq!(offset_y, 0);
  }

  #[test]
  fn test_stretch_different_aspect() {
    // 4:3 buffer stretched to 16:9 window
    let (offset_x, offset_y, scaled_w, scaled_h) =
      calculate_scaled_dimensions(800, 600, 1920, 1080, ScaleMode::Stretch);
    assert_eq!(scaled_w, 1920);
    assert_eq!(scaled_h, 1080);
    assert_eq!(offset_x, 0);
    assert_eq!(offset_y, 0);
  }

  // ============================================================================
  // Edge Cases
  // ============================================================================

  #[test]
  fn test_zero_dimensions() {
    // Zero buffer dimensions
    let (offset_x, offset_y, scaled_w, scaled_h) =
      calculate_scaled_dimensions(0, 0, 800, 600, ScaleMode::Fit);
    assert_eq!(scaled_w, 0);
    assert_eq!(scaled_h, 0);
    assert_eq!(offset_x, 400); // (800 - 0) / 2
    assert_eq!(offset_y, 300); // (600 - 0) / 2
  }

  #[test]
  fn test_zero_window_dimensions() {
    // Zero window dimensions
    let (offset_x, offset_y, scaled_w, scaled_h) =
      calculate_scaled_dimensions(800, 600, 0, 0, ScaleMode::Fit);
    assert_eq!(scaled_w, 0);
    assert_eq!(scaled_h, 0);
    assert_eq!(offset_x, 0);
    assert_eq!(offset_y, 0);
  }

  #[test]
  fn test_very_small_dimensions() {
    // 1x1 pixel
    let (offset_x, offset_y, scaled_w, scaled_h) =
      calculate_scaled_dimensions(1, 1, 100, 100, ScaleMode::Fit);
    assert_eq!(scaled_w, 100);
    assert_eq!(scaled_h, 100);
    assert_eq!(offset_x, 0);
    assert_eq!(offset_y, 0);
  }

  #[test]
  fn test_very_large_dimensions() {
    // 8K buffer to 4K window
    let (offset_x, offset_y, scaled_w, scaled_h) =
      calculate_scaled_dimensions(7680, 4320, 3840, 2160, ScaleMode::Fit);
    assert_eq!(scaled_w, 3840);
    assert_eq!(scaled_h, 2160);
    assert_eq!(offset_x, 0);
    assert_eq!(offset_y, 0);
  }

  #[test]
  fn test_extreme_aspect_ratio() {
    // Very wide buffer
    let (offset_x, offset_y, scaled_w, scaled_h) =
      calculate_scaled_dimensions(10000, 100, 800, 600, ScaleMode::Fit);
    // Scale limited by width: 800/10000 = 0.08
    assert_eq!(scaled_w, 800);
    assert_eq!(scaled_h, 8); // 100 * 0.08
    assert_eq!(offset_x, 0);
    assert_eq!(offset_y, 296); // (600 - 8) / 2
  }

  #[test]
  fn test_tall_aspect_ratio() {
    // Very tall buffer
    let (offset_x, offset_y, scaled_w, scaled_h) =
      calculate_scaled_dimensions(100, 10000, 800, 600, ScaleMode::Fit);
    // Scale limited by height: 600/10000 = 0.06
    assert_eq!(scaled_w, 6); // 100 * 0.06
    assert_eq!(scaled_h, 600);
    assert_eq!(offset_x, 397); // (800 - 6) / 2
    assert_eq!(offset_y, 0);
  }
}
