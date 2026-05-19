//! Cross-platform terminal escape sequences, painting, and control.
//!
//! - [`escape`] - raw ANSI/VT100 sequences and the [`VisualStyle`] → SGR bridge
//! - [`term_style`] - real terminal colours via [`TermStyle`](term_style::TermStyle)
//! - [`terminal_ext`] - cursor, screen, and terminal-size control

pub mod escape;
pub mod term_style;
pub mod terminal_ext;

pub use term_style::*;
