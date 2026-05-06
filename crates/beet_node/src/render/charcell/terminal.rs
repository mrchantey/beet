use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::component::Mutable;
use bevy::ecs::component::StorageType;
use std::io::BufWriter;
use std::io::Cursor;
use std::io::Read;
use std::io::Write;
use termion::async_stdin;
use termion::clear;
use termion::color;
use termion::cursor;
use termion::event::Event as TermionEvent;
use termion::input::TermRead;
use termion::screen;
use termion::style;

/// Input event from the terminal, targeted at the terminal entity.
#[derive(Debug, Clone, EntityTargetEvent)]
#[event(auto_propagate)]
pub struct TerminalInput(pub TermionEvent);

impl TerminalInput {
	/// Returns the inner event.
	pub fn event(&self) -> &TermionEvent { &self.0 }
}

/// Marker component: flush this terminal's writer each frame.
#[derive(Debug, Default, Component)]
pub struct StdioTerminal;

/// Marker resource: raw mode is enabled for the process terminal.
#[derive(Debug, Default, Resource)]
pub struct RawMode;

/// Recommended write buffer capacity for a full terminal frame (4 MiB).
pub const TERMINAL_BUFFER_CAPACITY: usize = 4 * 1024 * 1024;

/// Creates a [`BufWriter`] with [`TERMINAL_BUFFER_CAPACITY`], preventing mid-frame flushes and terminal flicker.
pub fn terminal_buf_writer<W: Write>(writer: W) -> BufWriter<W> {
	BufWriter::with_capacity(TERMINAL_BUFFER_CAPACITY, writer)
}

/// A terminal abstraction over a reader `R` and writer `W`.
///
/// Writes ANSI escape sequences via termion to `W`, reads input events from `R`.
/// Use [`StdoutTerminal`] for local stdio and [`BufferedTerminal`] for SSH/headless.
pub struct Terminal<R, W>
where
	R: Read + Send + 'static,
	W: Write + Send + Sync + 'static,
{
	/// Input reader.
	pub reader: R,
	/// Output writer — receives all ANSI escape sequences and cell data.
	pub writer: W,
	size: UVec2,
}

/// A local stdio terminal backed by async stdin and a buffered stdout.
pub type StdoutTerminal =
	Terminal<termion::AsyncReader, BufWriter<std::io::Stdout>>;

/// A headless terminal that buffers output in a [`Vec<u8>`]; suited for SSH or testing.
pub type BufferedTerminal = Terminal<Cursor<Vec<u8>>, Vec<u8>>;

// Manual Component impl to avoid requiring R: Component + W: Component bounds.
impl<R, W> Component for Terminal<R, W>
where
	R: Read + Send + 'static,
	W: Write + Send + Sync + 'static,
{
	type Mutability = Mutable;
	const STORAGE_TYPE: StorageType = StorageType::Table;
}

// Safety: Terminal is always accessed through exclusive ECS queries (`&mut Terminal`).
// The reader field is never exposed via shared references across thread boundaries.
unsafe impl<R, W> Sync for Terminal<R, W>
where
	R: Read + Send + 'static,
	W: Write + Send + Sync + 'static,
{
}

impl Terminal<termion::AsyncReader, BufWriter<std::io::Stdout>> {
	/// Create a terminal sized to the current stdout dimensions.
	pub fn new_stdout() -> Result<(Self, StdioTerminal)> {
		let size = termion::terminal_size()
			.map(|(w, h)| UVec2::new(w as u32, h as u32))
			.unwrap_or(UVec2::new(80, 24));
		let terminal = Terminal::new(
			async_stdin(),
			terminal_buf_writer(std::io::stdout()),
			size,
		);
		Ok((terminal, StdioTerminal))
	}
}

impl Terminal<Cursor<Vec<u8>>, Vec<u8>> {
	/// Create a headless terminal for SSH or testing with a known size.
	pub fn new_buffered(size: UVec2) -> Self {
		Terminal::new(Cursor::new(Vec::new()), Vec::new(), size)
	}

	/// Drain the output buffer.
	pub fn take_output(&mut self) -> Vec<u8> {
		core::mem::take(&mut self.writer)
	}
}

impl<R, W> Terminal<R, W>
where
	R: Read + Send + 'static,
	W: Write + Send + Sync + 'static,
{
	/// Create a terminal with explicit reader, writer, and size.
	pub fn new(reader: R, writer: W, size: UVec2) -> Self {
		Self {
			reader,
			writer,
			size,
		}
	}

	/// Current terminal size.
	pub fn size(&self) -> UVec2 { self.size }

	/// Update the terminal size.
	pub fn set_size(&mut self, size: UVec2) { self.size = size; }

	/// Parse all pending input events from the reader without blocking.
	pub fn read_events(&mut self) -> Vec<TermionEvent> {
		(&mut self.reader).events().filter_map(|e| e.ok()).collect()
	}

	/// Parse raw bytes as terminal input events.
	pub fn parse_bytes(bytes: &[u8]) -> Vec<TermionEvent> {
		bytes.events().filter_map(|e| e.ok()).collect()
	}

	/// Enter the alternate screen buffer.
	pub fn enter_alternate_screen(&mut self) -> Result {
		write!(self.writer, "{}", screen::ToAlternateScreen)
			.map_err(|e| bevyhow!("{e}"))
	}

	/// Return to the main screen buffer.
	pub fn leave_alternate_screen(&mut self) -> Result {
		write!(self.writer, "{}", screen::ToMainScreen)
			.map_err(|e| bevyhow!("{e}"))
	}

	/// Move cursor to (col, row), 0-indexed.
	pub fn goto(&mut self, col: u32, row: u32) -> Result {
		write!(
			self.writer,
			"{}",
			cursor::Goto((col + 1) as u16, (row + 1) as u16)
		)
		.map_err(|e| bevyhow!("{e}"))
	}

	/// Set foreground colour via 24-bit RGB.
	pub fn set_fg(&mut self, r: u8, g: u8, b: u8) -> Result {
		write!(self.writer, "{}", color::Fg(color::Rgb(r, g, b)))
			.map_err(|e| bevyhow!("{e}"))
	}

	/// Set background colour via 24-bit RGB.
	pub fn set_bg(&mut self, r: u8, g: u8, b: u8) -> Result {
		write!(self.writer, "{}", color::Bg(color::Rgb(r, g, b)))
			.map_err(|e| bevyhow!("{e}"))
	}

	/// Reset all SGR attributes.
	pub fn reset_style(&mut self) -> Result {
		write!(self.writer, "{}", style::Reset).map_err(|e| bevyhow!("{e}"))
	}

	/// Write text attribute SGR codes from a [`VisualStyle`].
	fn write_text_attrs(
		&mut self,
		vstyle: &crate::style::VisualStyle,
	) -> Result {
		use crate::style::TextStyle;
		let ts = vstyle.text_style;
		// dim is derived from foreground alpha, not a flag
		if let Some(fg) = vstyle.foreground {
			if fg.to_srgba_u8().alpha < 128 {
				write!(self.writer, "{}", style::Faint)
					.map_err(|e| bevyhow!("{e}"))?;
			}
		}
		if ts.contains(TextStyle::BOLD) {
			write!(self.writer, "{}", style::Bold)
				.map_err(|e| bevyhow!("{e}"))?;
		}
		if ts.contains(TextStyle::ITALIC) {
			write!(self.writer, "{}", style::Italic)
				.map_err(|e| bevyhow!("{e}"))?;
		}
		if ts.contains(TextStyle::UNDERLINE) {
			write!(self.writer, "{}", style::Underline)
				.map_err(|e| bevyhow!("{e}"))?;
		}
		if ts.contains(TextStyle::BLINK) {
			write!(self.writer, "{}", style::Blink)
				.map_err(|e| bevyhow!("{e}"))?;
		}
		if ts.contains(TextStyle::REVERSED) {
			write!(self.writer, "{}", style::Invert)
				.map_err(|e| bevyhow!("{e}"))?;
		}
		if ts.contains(TextStyle::LINE_THROUGH) {
			write!(self.writer, "{}", style::CrossedOut)
				.map_err(|e| bevyhow!("{e}"))?;
		}
		// RAPID_BLINK, HIDDEN, OVERLINE have no termion equivalents; use raw CSI
		if ts.contains(TextStyle::RAPID_BLINK) {
			self.writer
				.write_all(b"\x1b[6m")
				.map_err(|e| bevyhow!("{e}"))?;
		}
		if ts.contains(TextStyle::HIDDEN) {
			self.writer
				.write_all(b"\x1b[8m")
				.map_err(|e| bevyhow!("{e}"))?;
		}
		if ts.contains(TextStyle::OVERLINE) {
			self.writer
				.write_all(b"\x1b[53m")
				.map_err(|e| bevyhow!("{e}"))?;
		}
		Ok(())
	}
}

impl<R, W> super::Backend for Terminal<R, W>
where
	R: Read + Send + 'static,
	W: Write + Send + Sync + 'static,
{
	fn hide_cursor(&mut self) -> Result {
		write!(self.writer, "{}", cursor::Hide).map_err(|e| bevyhow!("{e}"))
	}

	fn show_cursor(&mut self) -> Result {
		write!(self.writer, "{}", cursor::Show).map_err(|e| bevyhow!("{e}"))
	}

	fn get_cursor(&mut self) -> Result<UVec2> { Ok(UVec2::ZERO) }

	fn set_cursor(&mut self, position: UVec2) -> Result {
		self.goto(position.x, position.y)
	}

	fn clear(&mut self) -> Result {
		write!(self.writer, "{}{}", clear::All, cursor::Goto(1, 1))
			.map_err(|e| bevyhow!("{e}"))
	}

	fn window_size(&mut self) -> Result<super::WindowSize> {
		Ok(super::WindowSize {
			chars: self.size,
			pixels: UVec2::ZERO,
		})
	}

	fn draw<'a>(
		&mut self,
		cells: impl IntoIterator<Item = (UVec2, &'a super::Cell)>,
	) -> Result {
		let mut last_pos: Option<UVec2> = None;
		for (pos, cell) in cells {
			// skip Goto when directly following the previous cell
			let skip = matches!(last_pos, Some(lp) if pos.x == lp.x + 1 && pos.y == lp.y);
			if !skip {
				self.goto(pos.x, pos.y)?;
			}
			last_pos = Some(pos);
			if let Some(fg) = cell.style.foreground {
				let c = fg.to_srgba_u8();
				self.set_fg(c.red, c.green, c.blue)?;
			}
			if let Some(bg) = cell.style.background {
				let c = bg.to_srgba_u8();
				self.set_bg(c.red, c.green, c.blue)?;
			}
			self.write_text_attrs(&cell.style)?;
			write!(self.writer, "{}", cell.symbol)
				.map_err(|e| bevyhow!("{e}"))?;
			self.reset_style()?;
		}
		Ok(())
	}

	fn flush(&mut self) -> Result {
		self.writer.flush().map_err(|e| bevyhow!("{e}"))
	}
}

// ── Systems ───────────────────────────────────────────────────────────────────

/// Render changed [`CharcellRenderer`] buffers into their terminal's writer.
pub fn render_terminal<R, W>(
	mut query: Populated<
		(&mut Terminal<R, W>, &CharcellRenderer),
		Changed<CharcellRenderer>,
	>,
) -> Result
where
	R: Read + Send + 'static,
	W: Write + Send + Sync + 'static,
{
	use super::Backend;
	for (mut terminal, renderer) in query.iter_mut() {
		terminal.enter_alternate_screen()?;
		terminal.hide_cursor()?;
		terminal.draw(renderer.iter_cells())?;
	}
	Ok(())
}

/// Flush all terminal writers in PostUpdate to avoid partial frames.
pub fn flush_terminals<R, W>(mut query: Query<&mut Terminal<R, W>>) -> Result
where
	R: Read + Send + 'static,
	W: Write + Send + Sync + 'static,
{
	use super::Backend;
	for mut terminal in query.iter_mut() {
		terminal.flush()?;
	}
	Ok(())
}

/// Restore terminals (leave alternate screen, show cursor) on shutdown.
pub fn restore_terminals<R, W>(mut query: Query<&mut Terminal<R, W>>) -> Result
where
	R: Read + Send + 'static,
	W: Write + Send + Sync + 'static,
{
	use super::Backend;
	for mut terminal in query.iter_mut() {
		terminal.leave_alternate_screen()?;
		terminal.show_cursor()?;
		terminal.flush()?;
	}
	Ok(())
}

// ── Raw mode ──────────────────────────────────────────────────────────────────

/// Enable raw mode for the process terminal.
pub fn enable_raw_mode() -> Result { raw_mode_impl::enable() }

/// Disable raw mode for the process terminal.
pub fn disable_raw_mode() -> Result { raw_mode_impl::disable() }

/// Disable raw mode when [`AppExit`] fires.
pub fn try_disable_raw_mode(res: Option<Res<RawMode>>) -> Result {
	if res.is_some() {
		raw_mode_impl::disable()?;
	}
	Ok(())
}

#[cfg(unix)]
mod raw_mode_impl {
	use beet_core::prelude::*;
	use std::io::Stdout;
	use std::sync::Mutex;
	use termion::raw::IntoRawMode;
	use termion::raw::RawTerminal;

	// Safety: RawTerminal<Stdout> is Send+Sync because Stdout and Termios are both Send+Sync.
	static GUARD: Mutex<Option<RawTerminal<Stdout>>> = Mutex::new(None);

	pub fn enable() -> Result {
		let mut guard = GUARD.lock().unwrap();
		if guard.is_none() {
			*guard = Some(
				std::io::stdout()
					.into_raw_mode()
					.map_err(|e| bevyhow!("enable_raw_mode: {e}"))?,
			);
		}
		Ok(())
	}

	pub fn disable() -> Result {
		GUARD.lock().unwrap().take();
		Ok(())
	}
}

#[cfg(not(unix))]
mod raw_mode_impl {
	use beet_core::prelude::*;

	pub fn enable() -> Result {
		bevybail!("raw mode not supported on this platform")
	}

	pub fn disable() -> Result {
		bevybail!("raw mode not supported on this platform")
	}
}
