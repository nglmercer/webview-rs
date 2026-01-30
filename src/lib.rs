#![deny(clippy::all)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Webview N-API Bindings
//!
//! This library provides N-API bindings for using tao and wry
//! in Node.js applications. All methods, APIs, enums, and types are exported
//! directly for Node.js composition.

// Wry bindings
pub mod wry;

// Winit bindings
pub mod winit;

// Re-export wry types
pub use wry::enums::{
  BackgroundThrottlingPolicy, DragDropEvent, Error, NewWindowResponse, PageLoadEvent, ProxyConfig,
  WryTheme,
};
pub use wry::functions::webview_version;
pub use wry::structs::{
  InitializationScript, NewWindowFeatures, NewWindowOpener, ProxyEndpoint, Rect,
  RequestAsyncResponder, WebContext, WebView, WebViewAttributes, WebViewBuilder,
};
pub use wry::types::{Result, WebViewId, RGBA};

// Re-export winit types
pub use winit::enums::{
  CursorIcon, DeviceEvent, ElementState, Force, Key, KeyCode, KeyLocation, ModifiersState,
  MouseButton, MouseButtonState, ResizeDirection, ScaleMode, StartCause, TouchPhase, WindowEvent,
  WinitControlFlow, WinitFullscreenType, WinitTheme,
};
pub use winit::functions::{available_monitors, primary_monitor, winit_version};
pub use winit::structs::{
  CursorPosition, EventLoop, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget, GestureEvent,
  HiDpiScaling, Icon, KeyboardEvent, MonitorInfo, MouseEvent, NotSupportedError, OsError, Position,
  RawKeyEvent, Rectangle, ResizeDetails, ScaleFactorChangeDetails, Size, ThemeChangeDetails, Touch,
  VideoMode, Window, WindowAttributes, WindowBuilder, WindowDragOptions, WindowJumpOptions,
  WindowOptions, WindowSizeConstraints, WinitProgressBar,
};
pub use winit::types::{
  AxisId, ButtonId, DeviceId, Result as WinitResult, WindowId, RGBA as WinitRGBA,
};

// Re-export render types
pub use winit::render::{render_pixels, PixelRenderer, RenderOptions};

// High-level API adapter
pub mod high_level;
pub use high_level::*;
