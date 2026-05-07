use crate::prelude::*;
use beet_core::exports::async_channel;
use beet_core::prelude::*;
use std::io;
use std::io::BufWriter;
use std::io::Cursor;
use std::io::Read;
use std::io::Write;
use termion::clear;
use termion::color;
use termion::cursor;
use termion::event::Event as TermionEvent;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen;
use termion::style;



#[derive(Debug, Clone, SetWith, Component)]
#[component(on_add=Self::on_add)]
pub struct StdioTerminal {
	/// Whether to run the terminal in raw mode,
	/// defaults to true
	raw_mode: bool,
	/// When enabled, applies a ctrl+c and panic hook
	/// to ensure terminal state is restored
	restore_hook: bool,
	config: TerminalConfig,
}

impl Default for StdioTerminal {
	fn default() -> Self {
		Self {
			restore_hook: true,
			raw_mode: true,
			config: default(),
		}
	}
}


impl StdioTerminal {
	pub fn inline() -> Self {
		Self {
			raw_mode: false,
			config: TerminalConfig::inline(),
			..default()
		}
	}

	fn on_add(mut world: DeferredWorld, cx: HookContext) {
		let stdio = world.entity(cx.entity).get::<StdioTerminal>().unwrap();
		stdio.register_restore_hook().unwrap();
		/// Creates a [`BufWriter`] with [`TERMINAL_BUFFER_CAPACITY`], preventing mid-frame flushes and terminal flicker.
		const TERMINAL_BUFFER_CAPACITY: usize = 4 * 1024 * 1024;

		let size = termion::terminal_size()
			.map(|(w, h)| UVec2::new(w as u32, h as u32))
			.unwrap_or(UVec2::new(80, 24));
		let terminal = if stdio.raw_mode {
			Terminal::new(
				AsyncReader::default(),
				BufWriter::with_capacity(
					TERMINAL_BUFFER_CAPACITY,
					// RawMode struct will
					std::io::stdout().into_raw_mode().unwrap(),
				),
				size,
				stdio.config.clone(),
			)
		} else {
			Terminal::new(
				AsyncReader::default(),
				BufWriter::with_capacity(
					TERMINAL_BUFFER_CAPACITY,
					std::io::stdout(),
				),
				size,
				stdio.config.clone(),
			)
		};
		world.commands().entity(cx.entity).insert(terminal);
	}

	/// Ensures Terminal::restore_config is called in the
	/// event of a forced exit (ctrl+c or panic)
	fn register_restore_hook(&self) -> Result {
		if !self.restore_hook {
			return Ok(());
		}
		let config = self.config.clone();
		terminal_ext::on_force_exit(move || {
			match Terminal::new(
				io::empty(),
				io::stdout(),
				default(),
				config.clone(),
			)
			.restore_config()
			{
				Ok(_) => {
					// println!("terminal restored");
				}
				Err(err) => {
					eprintln!("Error restoring terminal state: {err}");
				}
			}
		})
	}
}

macro_rules! csi {
    ($( $l:expr ),*) => { concat!("\x1B[", $( $l ),*) };
}

#[derive(Debug, Clone, SetWith)]
pub struct TerminalConfig {
	/// Whether to use an alternate screen for the terminal,
	/// defaults to true
	alternate_screen: bool,
	/// Whether to hide the cursor when rendering, defaults to true
	hide_cursor: bool,
	enable_mouse: bool,
}

impl TerminalConfig {
	pub fn inline() -> Self {
		Self {
			alternate_screen: false,
			hide_cursor: false,
			..default()
		}
	}
}

impl Default for TerminalConfig {
	fn default() -> Self {
		Self {
			alternate_screen: true,
			hide_cursor: true,
			enable_mouse: true,
		}
	}
}


/// A terminal abstraction over a reader `R` and writer `W`.
///
/// Writes ANSI escape sequences via termion to `W`, reads input events from `R`.
/// Use [`StdoutTerminal`] for local stdio and [`BufferedTerminal`] for SSH/headless.
#[derive(Component)]
pub struct Terminal {
	/// Input reader.
	pub reader: Box<dyn 'static + Send + Sync + Read>,
	/// Output writer — receives all ANSI escape sequences and cell data.
	pub writer: Box<dyn 'static + Send + Sync + Write>,
	size: UVec2,
	/// Terminal configuration options, applied on initialization and immutable,
	/// also used on cleanup to restore previous state.
	config: TerminalConfig,
}

pub struct AsyncReader {
	recv: Receiver<io::Result<u8>>,
}
impl Default for AsyncReader {
	fn default() -> Self {
		let (send, recv) = async_channel::unbounded();
		std::thread::spawn(move || {
			for i in termion::get_tty().unwrap().bytes() {
				if send.try_send(i).is_err() {
					return;
				}
			}
		});

		Self { recv }
	}
}

impl Read for AsyncReader {
	fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
		let mut total = 0;

		loop {
			if total >= buf.len() {
				break;
			}

			match self.recv.try_recv() {
				Ok(Ok(b)) => {
					buf[total] = b;
					total += 1;
				}
				Ok(Err(e)) => return Err(e),
				Err(_) => break,
			}
		}

		Ok(total)
	}
}

impl Terminal {
	pub fn new_buffered(size: UVec2) -> Self {
		Self::new(Cursor::new(Vec::new()), Vec::new(), size, default())
	}
	// pub fn take_output(&mut self) -> Vec<u8> {
	// 	core::mem::take(&mut self.writer)
	// }
	/// Create a terminal with explicit reader, writer, and size.
	pub fn new(
		reader: impl 'static + Send + Sync + Read,
		writer: impl 'static + Send + Sync + Write,
		size: UVec2,
		config: TerminalConfig,
	) -> Self {
		let mut this = Self {
			reader: Box::new(reader),
			writer: Box::new(writer),
			config,
			size,
		};
		this.apply_config().unwrap();
		this
	}

	fn apply_config(&mut self) -> Result {
		if self.config.alternate_screen {
			write!(self.writer, "{}", screen::ToAlternateScreen)?;
		}
		if self.config.hide_cursor {
			write!(self.writer, "{}", cursor::Hide)?;
		}
		if self.config.enable_mouse {
			const ENTER_MOUSE_SEQUENCE: &'static str =
				csi!("?1000h\x1b[?1002h\x1b[?1015h\x1b[?1006h");
			write!(self.writer, "{}", ENTER_MOUSE_SEQUENCE)?;
		}
		Ok(())
	}
	fn restore_config(&mut self) -> Result {
		if self.config.alternate_screen {
			write!(self.writer, "{}", screen::ToMainScreen)?;
		}
		if self.config.hide_cursor {
			write!(self.writer, "{}", cursor::Show)?;
		}
		if self.config.enable_mouse {
			/// A sequence of escape codes to disable terminal mouse support.
			const EXIT_MOUSE_SEQUENCE: &'static str =
				csi!("?1006l\x1b[?1015l\x1b[?1002l\x1b[?1000l");
			write!(self.writer, "{}", EXIT_MOUSE_SEQUENCE)?;
		}

		Ok(())
	}

	/// Current terminal size.
	pub fn size(&self) -> UVec2 { self.size }

	/// Update the terminal size.
	pub fn set_size(&mut self, size: UVec2) {
		// wrong
		self.size = size;
	}

	/// Parse all pending input events from the reader without blocking.
	pub fn read_events(&mut self) -> Result<Vec<TermionEvent>> {
		self.reader
			.as_mut()
			.events()
			.xtry_map(|e| e.map_err(|err| err.into()))
	}

	/// Parse raw bytes as terminal input events.
	pub fn parse_bytes(bytes: &[u8]) -> Vec<TermionEvent> {
		bytes.events().filter_map(|e| e.ok()).collect()
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
		if vstyle.dim_foregeround() {
			write!(self.writer, "{}", style::Faint)
				.map_err(|e| bevyhow!("{e}"))?;
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

	// fn get_cursor(&mut self) -> Result<UVec2> { Ok(UVec2::ZERO) }

	// fn set_cursor(&mut self, position: UVec2) -> Result {
	// 	self.goto(position.x, position.y)
	// }

	pub fn clear(&mut self) -> Result {
		write!(self.writer, "{}{}", clear::All, cursor::Goto(1, 1))
			.map_err(|e| bevyhow!("{e}"))
	}

	// fn window_size(&mut self) -> Result<super::WindowSize> {
	// 	Ok(super::WindowSize {
	// 		chars: self.size,
	// 		pixels: UVec2::ZERO,
	// 	})
	// }

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
pub fn render_terminal(
	mut query: Populated<
		(&mut Terminal, &CharcellRenderer),
		Changed<CharcellRenderer>,
	>,
) -> Result {
	for (mut terminal, renderer) in query.iter_mut() {
		terminal.draw(renderer.iter_cells())?;
	}
	Ok(())
}

/// Flush all terminal writers in PostUpdate to avoid partial frames.
pub fn flush_terminals(mut query: Query<&mut Terminal>) -> Result {
	for mut terminal in query.iter_mut() {
		terminal.flush()?;
	}
	Ok(())
}

/// Restore terminals (leave alternate screen, show cursor) on shutdown.
pub fn restore_terminals(
	mut commands: Commands,
	mut query: Query<(Entity, &mut Terminal)>,
) -> Result {
	for (entity, mut terminal) in query.iter_mut() {
		terminal.restore_config()?;
		terminal.flush()?;
		// remove the terminal, running any drop steps like
		// restoring RawMode
		commands.entity(entity).remove::<Terminal>();
	}
	Ok(())
}
