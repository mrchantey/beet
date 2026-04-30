//! Cross-domain primitives for container and text layout,
//! based on CSS Flexbox.
//!
//! Also contains discrete coordinate system algorithms for
//! rendering to TUI and similar. These are used instead of
//! Ratatui for a few additional features:
//!
//! 1. Interactivity
//! - we need to know which entity was responsible for the cells,
//! for remapping on click
//!
//! 2. Multiline wrap
//! - we need wrapping support for buttons and other multiline elements
mod flex;
mod render;
mod text;
pub use flex::*;
pub use render::*;
pub use text::*;
