use crate::prelude::*;
use beet_core::prelude::*;
use ratatui::crossterm;
use std::io::Write;

/// Terminal backend that writes ANSI escape sequences to any [`Write`] target.
pub struct CrosstermBackend<W: Write> {
	writer: W,
}

impl<W: Write> CrosstermBackend<W> {
	pub fn new(writer: W) -> Self { Self { writer } }

	pub fn writer(&self) -> &W { &self.writer }

	pub fn writer_mut(&mut self) -> &mut W { &mut self.writer }
}
