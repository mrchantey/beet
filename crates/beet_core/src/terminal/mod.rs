//! Cross-platform terminal escape sequences, painting, and control.
//!
//! - [`escape`] - raw ANSI/VT100 escape sequences
//! - [`term_style`] - real terminal colours via [`TermStyle`](term_style::TermStyle)
//! - [`terminal_ext`] - cursor, screen, and terminal-size control

pub mod escape;
pub mod term_style;
// Cursor/screen/size control writes to stdout via crossterm/libc, so it is
// std-only; `escape` (raw sequences) and `term_style` (colour) stay no_std.
#[cfg(feature = "std")]
pub mod terminal_ext;

pub use term_style::*;
