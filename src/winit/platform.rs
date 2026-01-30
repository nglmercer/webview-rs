//! Platform detection and utilities
//!
//! This module provides utilities for detecting the current display server
//! and platform-specific configurations.

use std::env;

/// Display server type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayServer {
  /// X11 display server
  X11,
  /// Wayland display server
  Wayland,
  /// Unknown or other display server
  Unknown,
}

/// Platform information
#[derive(Debug, Clone)]
pub struct PlatformInfo {
  /// The display server type
  pub display_server: DisplayServer,
  /// Whether the platform supports transparency
  pub supports_transparency: bool,
  /// Whether the platform supports window positioning
  pub supports_positioning: bool,
  /// Whether the platform supports direct pixel buffer rendering
  pub supports_direct_rendering: bool,
}

impl Default for PlatformInfo {
  fn default() -> Self {
    Self::detect()
  }
}

impl PlatformInfo {
  /// Detects the current platform information
  pub fn detect() -> Self {
    let display_server = Self::detect_display_server();

    #[cfg(target_os = "linux")]
    {
      match display_server {
        DisplayServer::Wayland => PlatformInfo {
          display_server,
          supports_transparency: true,
          supports_positioning: false, // Wayland doesn't allow arbitrary positioning
          supports_direct_rendering: false, // Wayland requires different rendering approach
        },
        DisplayServer::X11 => PlatformInfo {
          display_server,
          supports_transparency: true,
          supports_positioning: true,
          supports_direct_rendering: true,
        },
        DisplayServer::Unknown => PlatformInfo {
          display_server,
          supports_transparency: false,
          supports_positioning: false,
          supports_direct_rendering: false,
        },
      }
    }

    #[cfg(not(target_os = "linux"))]
    {
      PlatformInfo {
        display_server: DisplayServer::Unknown,
        supports_transparency: cfg!(target_os = "macos") || cfg!(target_os = "windows"),
        supports_positioning: true,
        supports_direct_rendering: true,
      }
    }
  }

  /// Detects the current display server
  fn detect_display_server() -> DisplayServer {
    // Check WAYLAND_DISPLAY environment variable
    if env::var("WAYLAND_DISPLAY").is_ok() {
      return DisplayServer::Wayland;
    }

    // Check XDG_SESSION_TYPE environment variable
    if let Ok(session_type) = env::var("XDG_SESSION_TYPE") {
      if session_type.to_lowercase() == "wayland" {
        return DisplayServer::Wayland;
      } else if session_type.to_lowercase() == "x11" {
        return DisplayServer::X11;
      }
    }

    // Check DISPLAY environment variable (X11)
    if env::var("DISPLAY").is_ok() {
      return DisplayServer::X11;
    }

    DisplayServer::Unknown
  }

  /// Returns true if running on Wayland
  pub fn is_wayland(&self) -> bool {
    self.display_server == DisplayServer::Wayland
  }

  /// Returns true if running on X11
  pub fn is_x11(&self) -> bool {
    self.display_server == DisplayServer::X11
  }
}

/// Global platform information
pub fn platform_info() -> PlatformInfo {
  PlatformInfo::detect()
}
