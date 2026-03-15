//! Terminal UI module using ratatui.
//!
//! This module provides the TUI rendering and input handling:
//!
//! - `render`: Main frame rendering and layout
//! - `input`: Keyboard event handling
//! - `styles`: Color schemes and text styling
//! - `tabs`: Tab-specific content rendering (roster, events, etc.)

pub mod input;
pub mod render;
pub mod styles;
pub mod tabs;
