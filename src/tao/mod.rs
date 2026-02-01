//! Tao bindings module
//!
//! This module contains all N-API bindings for tao types, structs, enums, and functions.

pub mod enums;
pub mod functions;
pub mod platform;
pub mod render;
pub mod structs;
pub mod types;

// Re-export render module items for backward compatibility
pub use render::{render_pixels, PixelRenderer, RenderOptions};
