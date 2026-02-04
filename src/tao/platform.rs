//! Platform detection and utilities
//!
//! This module provides utilities for detecting the current display server
//! and platform-specific configurations.

use std::env;

/// Display server type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayServer {
  /// X11 display server (Linux/BSD)
  X11,
  /// Wayland Compositor (Modern Linux)
  Wayland,
  /// Windows Desktop Window Manager / Win32
  Windows,
  /// Apple Quartz Compositor (macOS)
  Quartz,
  /// Unknown or Headless (e.g. Server CLI)
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
    // --- LINUX CONFIGURATION ---
    #[cfg(target_os = "linux")]
    {
      // Priority 1: Check for Wayland
      // If WAYLAND_DISPLAY is set, we are likely running natively on Wayland.
      if env::var("WAYLAND_DISPLAY").is_ok() {
        return PlatformInfo {
          display_server: DisplayServer::Wayland,
          supports_transparency: true,
          // Wayland protocols explicitly discourage/block absolute window positioning
          // by the client for security reasons.
          supports_positioning: false,
          supports_direct_rendering: true,
        };
      }

      // Priority 2: Check for X11
      // If DISPLAY is set, we are on X11 (or XWayland without the Wayland var exposed).
      if env::var("DISPLAY").is_ok() {
        return PlatformInfo {
          display_server: DisplayServer::X11,
          supports_transparency: true,
          supports_positioning: true,
          supports_direct_rendering: true,
        };
      }

      // Priority 3: Headless / Console
      PlatformInfo {
        display_server: DisplayServer::Unknown,
        supports_transparency: false,
        supports_positioning: false,
        supports_direct_rendering: false,
      };
    }

    // --- WINDOWS CONFIGURATION ---
    #[cfg(target_os = "windows")]
    {
      PlatformInfo {
        display_server: DisplayServer::Windows,
        supports_transparency: true,
        supports_positioning: true,
        supports_direct_rendering: true,
      }
    }

    // --- MACOS CONFIGURATION ---
    #[cfg(target_os = "macos")]
    {
      PlatformInfo {
        display_server: DisplayServer::Quartz,
        supports_transparency: true,
        supports_positioning: true,
        supports_direct_rendering: true,
      }
    }

    // --- OTHERS (BSD, Android, iOS, etc.) ---
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
      PlatformInfo {
        display_server: DisplayServer::Unknown,
        supports_transparency: false,
        supports_positioning: false,
        supports_direct_rendering: false,
      }
    }
  }

  /// Returns true if running on X11
  pub fn is_x11(&self) -> bool {
    self.display_server == DisplayServer::X11
  }

  /// Returns true if running on Wayland
  pub fn is_wayland(&self) -> bool {
    self.display_server == DisplayServer::Wayland
  }

  /// Returns true if running on Windows
  pub fn is_windows(&self) -> bool {
    self.display_server == DisplayServer::Windows
  }

  /// Returns true if running on macOS
  pub fn is_macos(&self) -> bool {
    self.display_server == DisplayServer::Quartz
  }
}

/// Global platform information
pub fn platform_info() -> PlatformInfo {
  PlatformInfo::detect()
}
