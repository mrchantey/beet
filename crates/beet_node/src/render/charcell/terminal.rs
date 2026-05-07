use crate::prelude::*;
use beet_core::exports::async_channel;
use beet_core::exports::async_channel::Receiver;
use beet_core::prelude::*;
use std::io;
use std::io::BufWriter;
use std::io::Cursor;
use std::io::Read;
use std::io::Write;

use super::escape;

// ── StdioTerminal ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, SetWith, Component)]
#[component(on_add=Self::on_add)]
pub struct StdioTerminal {
	/// In raw mode, listen for ctrl+c events and exit the application.
	/// Does nothing in cooked mode, ctrl+c would directly exit the process or
	/// be handled by the restore_hook
	ctrl_c_exit: bool,
	/// When enabled, applies a ctrl+c and panic hook to restore terminal state.
	restore_hook: bool,
	config: TerminalConfig,
}

impl Default for StdioTerminal {
	fn default() -> Self {
		Self {
			restore_hook: true,
			ctrl_c_exit: true,
			config: TerminalConfig::default().with_raw_mode(true),
		}
	}
}

impl StdioTerminal {
	pub fn inline() -> Self {
		Self {
			config: TerminalConfig::inline(),
			..default()
		}
	}

	fn on_add(mut world: DeferredWorld, cx: HookContext) {
		let stdio = world.entity(cx.entity).get::<StdioTerminal>().unwrap();
		stdio.register_restore_hook().unwrap();
		/// Large write buffer prevents mid-frame flushes and terminal flicker.
		const TERMINAL_BUFFER_CAPACITY: usize = 4 * 1024 * 1024;

		let size = terminal_ext::size().unwrap_or(UVec2::new(80, 24));
		let terminal = Terminal::new(
			AsyncReader::stdin(),
			BufWriter::with_capacity(
				TERMINAL_BUFFER_CAPACITY,
				std::io::stdout(),
			),
			size,
			stdio.config.clone(),
		);
		world.commands().entity(cx.entity).insert(terminal);
	}

	/// Registers a hook that restores the terminal on ctrl+c or panic.
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
				Ok(_) => {}
				Err(err) => {
					eprintln!("Error restoring terminal state: {err}");
				}
			}
		})
	}
}

// ── TerminalConfig ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, SetWith)]
pub struct TerminalConfig {
	/// Whether to enable raw mode, this option applies for the terminal
	/// of the current process, and should not be used for remote terminals.
	raw_mode: bool,
	/// Whether to use the alternate screen buffer, defaults to true.
	alternate_screen: bool,
	/// Whether to hide the cursor when rendering, defaults to true.
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
			raw_mode: false,
			alternate_screen: true,
			hide_cursor: true,
			enable_mouse: true,
		}
	}
}

#[derive(Component)]
pub struct ChannelTerminal {
	write_recv: Receiver<Result<Vec<u8>>>,
	read_send: Sender<io::Result<u8>>,
}

impl ChannelTerminal {
	pub fn new(size: UVec2, config: TerminalConfig) -> (Self, Terminal) {
		let (writer, write_recv) = AsyncWriter::new();
		let (read_send, reader) = AsyncReader::new();
		(
			Self {
				write_recv,
				read_send,
			},
			Terminal::new(reader, writer, size, config),
		)
	}
	/// Send input to the terminals Reader,
	/// ie keyboard, mouse movements
	// enforce mut for bevy change detection
	pub fn send_input(&mut self, data: &[u8]) -> Result {
		for byte in data {
			self.read_send.try_send(Ok(*byte))?;
		}
		Ok(())
	}

	/// Receive output from the terminals Writer,
	/// ie clear screen, write to cell
	// enforce mut for bevy change detection
	pub fn drain_write(&mut self) -> Vec<u8> {
		let mut out = Vec::new();
		while let Ok(Ok(items)) = self.write_recv.try_recv() {
			out.extend(items);
		}
		out
	}
}


// ── AsyncReader ───────────────────────────────────────────────────────────────

/// Non-blocking reader that drains a background thread reading from `/dev/tty`.
pub struct AsyncReader {
	recv: Receiver<io::Result<u8>>,
}

impl AsyncReader {
	pub fn new() -> (Sender<io::Result<u8>>, Self) {
		let (send, recv) = async_channel::unbounded();
		(send, Self { recv })
	}


	pub fn stdin() -> Self {
		let (send, recv) = async_channel::unbounded();
		std::thread::spawn(move || {
			use std::io::Read;
			// Open /dev/tty directly so keyboard input is available
			// even when stdin is redirected.
			let Ok(tty) = std::fs::File::open("/dev/tty") else {
				return;
			};
			for byte in tty.bytes() {
				if send.try_send(byte).is_err() {
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




pub struct AsyncWriter {
	send: Sender<Result<Vec<u8>>>,
}

impl AsyncWriter {
	pub fn new() -> (Self, Receiver<Result<Vec<u8>>>) {
		let (send, recv) = async_channel::unbounded();
		(Self { send }, recv)
	}
}

impl Write for AsyncWriter {
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		self.send
			.try_send(Ok(buf.to_vec()))
			.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
		Ok(buf.len())
	}

	fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

// ── Terminal ──────────────────────────────────────────────────────────────────

/// A terminal abstraction over a reader `R` and writer `W`.
///
/// Writes ANSI escape sequences to `W` and reads input events via [`InputParser`].
/// Use [`StdioTerminal`] for local stdio and [`BufferedTerminal`] for SSH/headless.
#[derive(Component)]
pub struct Terminal {
	/// Input reader.
	pub reader: Box<dyn 'static + Send + Sync + Read>,
	/// Output writer — receives all ANSI escape sequences and cell data.
	pub writer: Box<dyn 'static + Send + Sync + Write>,
	size: UVec2,
	/// Configuration applied on init and used to restore previous state on exit.
	config: TerminalConfig,
	input_parser: InputParser,
}

impl Terminal {
	/// Create an in-memory [`Terminal`] backed by a [`Vec<u8>`] writer.
	pub fn new_buffered(size: UVec2) -> Self {
		Self::new(Cursor::new(Vec::new()), Vec::new(), size, default())
	}

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
			input_parser: InputParser::new(),
		};
		this.apply_config().unwrap();
		this
	}

	fn apply_config(&mut self) -> Result {
		if self.config.raw_mode {
			raw_mode::enable()?;
		}
		if self.config.alternate_screen {
			self.writer.write_all(escape::ENTER_ALT_SCREEN.as_bytes())?;
		}
		if self.config.hide_cursor {
			self.writer.write_all(escape::HIDE_CURSOR.as_bytes())?;
		}
		if self.config.enable_mouse {
			self.writer.write_all(escape::ENTER_MOUSE.as_bytes())?;
		}
		Ok(())
	}

	fn restore_config(&mut self) -> Result {
		if self.config.enable_mouse {
			self.writer.write_all(escape::LEAVE_MOUSE.as_bytes())?;
		}
		if self.config.hide_cursor {
			self.writer.write_all(escape::SHOW_CURSOR.as_bytes())?;
		}
		if self.config.alternate_screen {
			self.writer.write_all(escape::LEAVE_ALT_SCREEN.as_bytes())?;
		}
		if self.config.raw_mode {
			raw_mode::disable()?;
		}
		Ok(())
	}

	/// Current terminal size.
	pub fn size(&self) -> UVec2 { self.size }

	/// Update the terminal size.
	pub fn set_size(&mut self, size: UVec2) { self.size = size; }

	/// Read all pending input events without blocking.
	pub fn read_events(&mut self) -> Result<Vec<TerminalEvent>> {
		let mut buf = [0u8; 4096];
		let mut all_bytes = Vec::new();
		// Drain all available bytes from the non-blocking reader.
		loop {
			let n = self.reader.read(&mut buf)?;
			if n == 0 {
				break;
			}
			all_bytes.extend_from_slice(&buf[..n]);
		}
		self.input_parser.parse(&all_bytes)
	}

	/// Parse raw bytes as terminal input events without retaining state.
	// remove!
	pub fn parse_bytes(bytes: &[u8]) -> Vec<TerminalEvent> {
		InputParser::new().parse(bytes).unwrap_or_default()
	}

	/// Move cursor to 0-indexed `(col, row)`.
	pub fn goto(&mut self, col: u32, row: u32) -> Result {
		escape::write_goto(&mut self.writer, col, row)?.xok()
	}

	/// Set foreground colour via 24-bit RGB.
	pub fn set_fg(&mut self, r: u8, g: u8, b: u8) -> Result {
		escape::write_fg(&mut self.writer, r, g, b)?.xok()
	}

	/// Set background colour via 24-bit RGB.
	pub fn set_bg(&mut self, r: u8, g: u8, b: u8) -> Result {
		escape::write_bg(&mut self.writer, r, g, b)?.xok()
	}

	/// Reset all SGR attributes.
	pub fn reset_style(&mut self) -> Result {
		write!(self.writer, "{}", escape::RESET)?.xok()
	}

	/// Write text attribute SGR codes from a [`VisualStyle`].
	fn write_text_attrs(
		&mut self,
		vstyle: &crate::style::VisualStyle,
	) -> Result {
		use crate::style::TextStyle;
		let ts = vstyle.text_style;
		if vstyle.dim_foregeround() {
			write!(self.writer, "{}", escape::FAINT)?;
		}
		if ts.contains(TextStyle::BOLD) {
			write!(self.writer, "{}", escape::BOLD)?;
		}
		if ts.contains(TextStyle::ITALIC) {
			write!(self.writer, "{}", escape::ITALIC)?;
		}
		if ts.contains(TextStyle::UNDERLINE) {
			write!(self.writer, "{}", escape::UNDERLINE)?;
		}
		if ts.contains(TextStyle::BLINK) {
			write!(self.writer, "{}", escape::BLINK)?;
		}
		if ts.contains(TextStyle::REVERSED) {
			write!(self.writer, "{}", escape::INVERT)?;
		}
		if ts.contains(TextStyle::LINE_THROUGH) {
			write!(self.writer, "{}", escape::CROSSED_OUT)?;
		}
		if ts.contains(TextStyle::RAPID_BLINK) {
			write!(self.writer, "{}", escape::RAPID_BLINK)?;
		}
		if ts.contains(TextStyle::HIDDEN) {
			write!(self.writer, "{}", escape::HIDDEN)?;
		}
		if ts.contains(TextStyle::OVERLINE) {
			write!(self.writer, "{}", escape::OVERLINE)?;
		}
		Ok(())
	}

	/// Erase the screen and move the cursor to the home position.
	pub fn clear(&mut self) -> Result {
		write!(self.writer, "{}{}", escape::ERASE_ALL, escape::CURSOR_HOME)?
			.xok()
	}

	fn draw<'a>(
		&mut self,
		cells: impl IntoIterator<Item = (UVec2, &'a super::Cell)>,
	) -> Result {
		let mut last_pos: Option<UVec2> = None;
		for (pos, cell) in cells {
			// Skip goto when directly following the previous cell.
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
			write!(self.writer, "{}", cell.symbol)?;
			self.reset_style()?;
		}
		Ok(())
	}

	fn flush(&mut self) -> Result { self.writer.flush()?.xok() }
}

// ── BufferedTerminal ──────────────────────────────────────────────────────────

/// In-memory terminal for SSH or headless rendering.
///
/// All writes go to an internal [`Vec<u8>`] buffer that can be drained with
/// [`BufferedTerminal::take_output`]. Input bytes are parsed on demand via
/// [`BufferedTerminal::parse_bytes`].
#[derive(Component)]
pub struct BufferedTerminal {
	/// Direct access to the output buffer — write ANSI sequences here.
	pub writer: Vec<u8>,
	size: UVec2,
}

impl BufferedTerminal {
	/// Create a new [`BufferedTerminal`] with the given size.
	pub fn new_buffered(size: UVec2) -> Self {
		Self {
			writer: Vec::new(),
			size,
		}
	}

	/// Current terminal size.
	pub fn size(&self) -> UVec2 { self.size }

	/// Drain and return all buffered output bytes.
	pub fn take_output(&mut self) -> Vec<u8> {
		core::mem::take(&mut self.writer)
	}

	/// Parse raw bytes into [`TerminalEvent`]s without retaining state.
	pub fn parse_bytes(bytes: &[u8]) -> Vec<TerminalEvent> {
		Terminal::parse_bytes(bytes)
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
pub fn restore_terminals(mut query: Query<&mut Terminal>) -> Result {
	for mut terminal in query.iter_mut() {
		terminal.restore_config()?;
		terminal.flush()?;
	}
	Ok(())
}

#[cfg(target_arch = "wasm32")]
mod raw_mode {
	pub(super) fn enable() -> Result {
		bevybail!("raw mode not supported on this platform")
	}
	pub(super) fn disable() -> Result {
		bevybail!("raw mode not supported on this platform")
	}
}

#[cfg(not(target_arch = "wasm32"))]
mod raw_mode {
	use super::*;
	pub(super) fn enable() -> Result {
		crossterm::terminal::enable_raw_mode()?.xok()
	}
	pub(super) fn disable() -> Result {
		crossterm::terminal::disable_raw_mode()?.xok()
	}
}

/// Exit on ctrl+c when [`StdioTerminal::ctrl_c_exit`] is set.
pub fn exit_ctrl_c(
	ev: On<TerminalEvent>,
	mut commands: Commands,
	query: Query<&StdioTerminal>,
) {
	if let Ok(term) = query.get(ev.target()) {
		if term.ctrl_c_exit {
			if matches!(ev.event(), TerminalEvent::Key(k) if k == &KeyPress::CTRL_C)
			{
				commands.write_message(AppExit::Success);
			}
		}
	}
}
