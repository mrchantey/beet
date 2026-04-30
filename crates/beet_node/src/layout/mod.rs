//! Cross-domain primitives for container and text layout,
//! based on CSS Flexbox.
//!
//! # Overview
//!
//! This module provides a complete flexbox layout system using bevy's
//! coordinate types ([`UVec2`] and [`URect`]) for measuring and positioning.
//! Layout is performed in two passes:
//!
//! 1. **Measure**: bottom-up calculation of natural sizes
//! 2. **Layout**: top-down assignment of final positions, writing to a [`Buffer`]
//!
//! # Features
//!
//! - **FlexBox**: full CSS flexbox implementation with:
//!   - justify-content (start, center, end, space-between, space-around, space-evenly)
//!   - align-items and align-content
//!   - row and column gaps
//!   - wrapping support
//! - **LayoutStyle**: per-item styling with:
//!   - flex-order for reordering children
//!   - flex-grow for distributing free space
//!   - align-self for individual alignment
//!   - padding and margin (not yet fully implemented)
//! - **TextWidget**: multi-line text with word wrapping and alignment
//! - **Bordered**: wraps a child widget in a border
//! - **Buffer**: cell-based rendering with optional styling (foreground, background, underline)
//!
//! # Rendering
//!
//! Uses a discrete coordinate system for TUI and similar renderers.
//! Custom rendering instead of Ratatui provides:
//!
//! 1. **Interactivity**: track which entity rendered each cell for click remapping
//! 2. **Multiline wrap**: proper wrapping for bordered elements and text
//! 3. **Styling**: per-cell visual styling with colors and effects
mod bordered;
mod flex;
mod render;
mod text;
pub use bordered::*;
pub use flex::*;
pub use render::*;
pub use text::*;
