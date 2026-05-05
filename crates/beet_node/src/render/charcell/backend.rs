//! Charcell backends for interacting with the terminal,
//! This is directly influenced by, and intended to be compatible with ratatui backends.
use crate::render::Buffer;
use beet_core::prelude::*;
mod test_backend;
pub use test_backend::*;
#[cfg(feature = "tui")]
mod ratatui_backend;
#[cfg(feature = "tui")]
pub use ratatui_backend::*;
#[cfg(feature = "crossterm")]
mod crossterm_backend;
#[cfg(feature = "crossterm")]
pub use crossterm_backend::*;

/// The window size in characters (columns / rows) as well as pixels.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct WindowSize {
	/// Size of the window in characters (columns / rows).
	pub chars: UVec2,
	/// Size of the window in pixels.
	pub pixels: UVec2,
}

pub trait Backend {
	fn hide_cursor(&mut self) -> Result;
	fn show_cursor(&mut self) -> Result;
	fn get_cursor(&mut self) -> Result<UVec2>;
	fn set_cursor(&mut self, position: UVec2) -> Result;
	fn clear(&mut self) -> Result;
	fn window_size(&mut self) -> Result<WindowSize>;
	fn draw(&mut self, buffer: &Buffer) -> Result;
	fn flush(&mut self) -> Result;
}
