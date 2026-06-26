use crate::prelude::*;
use crate::style::VisualStyle;
use beet_core::exports::async_channel;
use beet_core::exports::async_channel::Receiver;
use beet_core::exports::async_channel::Sender;
use beet_core::prelude::*;
use std::io;
use std::io::BufWriter;
use std::io::Cursor;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

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
	/// When set, frames render to `/dev/tty` and the process `stdout`/`stderr`
	/// (logs, panics, stray prints) redirect to this file, so diagnostics never
	/// corrupt the alternate screen. `None` renders to `stdout` as-is (eg for
	/// inline mode). Relative paths resolve against the working directory.
	log_file: Option<PathBuf>,
	config: TerminalConfig,
}

impl Default for StdioTerminal {
	fn default() -> Self {
		Self {
			restore_hook: true,
			ctrl_c_exit: true,
			log_file: Some(PathBuf::from("target/beet-log.txt")),
			config: TerminalConfig::default().with_raw_mode(true),
		}
	}
}

impl StdioTerminal {
	pub fn inline() -> Self {
		Self {
			config: TerminalConfig::inline(),
			// inline mode shares the live screen, so it keeps writing to stdout
			// rather than splitting frames onto /dev/tty.
			log_file: None,
			..default()
		}
	}

	fn on_add(mut world: DeferredWorld, cx: HookContext) {
		let stdio = world.entity(cx.entity).get::<StdioTerminal>().unwrap();
		// kitty-graphics support is per-surface; a local stdio terminal detects it
		// from the process env (an SSH session detects from its pty instead). Bundled
		// with the Terminal insert below so the `stdio` borrow above is untouched.
		let graphics = KittyGraphicsSupport::default();
		// `BEET_HEADLESS` (tests, CI, automation): a buffered terminal that never
		// touches the real tty (no raw mode, alt screen, mouse, or restore hook), so
		// a scene that declares a `StdioTerminal` still spawns and reduces without
		// taking over or leaking escapes into the controlling terminal.
		if env_ext::var("BEET_HEADLESS").is_ok() {
			world
				.commands()
				.entity(cx.entity)
				.insert((Terminal::new_buffered(), graphics));
			return;
		}
		// best-effort: a process needs only one restore hook, so a second terminal
		// (eg several spawned across headless tests) reuses the first rather than
		// panicking. The real single-terminal app registers cleanly.
		if let Err(err) = stdio.register_restore_hook() {
			warn!("terminal restore hook not registered: {err}");
		}
		/// Large write buffer prevents mid-frame flushes and terminal flicker.
		const TERMINAL_BUFFER_CAPACITY: usize = 4 * 1024 * 1024;

		// when a log file is configured, render frames to /dev/tty and redirect
		// stdout/stderr to the file; if /dev/tty or the redirect is unavailable
		// (eg a sandbox), fall back to stdout so the app still runs.
		let writer: Box<dyn 'static + Send + Sync + Write> = stdio
			.log_file
			.clone()
			.and_then(|path| {
				let tty = terminal_ext::tty_writer().ok()?;
				terminal_ext::redirect_std_to_file(&path).ok()?;
				Some(Box::new(tty) as Box<dyn 'static + Send + Sync + Write>)
			})
			.unwrap_or_else(|| Box::new(std::io::stdout()));

		let terminal = Terminal::new(
			AsyncReader::stdin(),
			BufWriter::with_capacity(TERMINAL_BUFFER_CAPACITY, writer),
			stdio.config.clone(),
		);
		world
			.commands()
			.entity(cx.entity)
			.insert((terminal, graphics));
	}

	/// Registers a hook that restores the terminal on ctrl+c or panic.
	fn register_restore_hook(&self) -> Result {
		if !self.restore_hook {
			return Ok(());
		}
		let config = self.config.clone();
		// when std is redirected to a log file, the restore escapes must target
		// /dev/tty rather than the (redirected) stdout.
		let to_tty = self.log_file.is_some();
		terminal_ext::on_force_exit(move || {
			let result = if to_tty {
				terminal_ext::tty_writer().map_err(Into::into).and_then(
					|mut tty| {
						Terminal::restore_config_direct(&config, &mut tty)
					},
				)
			} else {
				Terminal::restore_config_direct(&config, &mut io::stdout())
			};
			if let Err(err) = result {
				eprintln!("Error restoring terminal state: {err}");
			}
		})
	}
}

// ── TerminalConfig ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, SetWith)]
pub struct TerminalConfig {
	/// Whether to enable raw mode; applies to the current process terminal only.
	/// Do not use for remote terminals.
	raw_mode: bool,
	/// Whether to use the alternate screen buffer, defaults to true.
	alternate_screen: bool,
	/// Whether to hide the cursor when rendering, defaults to true.
	hide_cursor: bool,
	enable_mouse: bool,
	/// Enable bracketed paste mode for structured [`TerminalEvent::Paste`] events.
	bracketed_paste: bool,
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
			bracketed_paste: false,
		}
	}
}

#[derive(Component)]
pub struct ChannelTerminal {
	write_recv: Receiver<Result<Vec<u8>>>,
	read_send: Sender<io::Result<u8>>,
}

impl ChannelTerminal {
	pub fn new(config: TerminalConfig) -> (Self, Terminal) {
		let (writer, write_recv) = AsyncWriter::new();
		let (read_send, reader) = AsyncReader::new();
		(
			Self {
				write_recv,
				read_send,
			},
			Terminal::new(reader, writer, config),
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

/// A terminal abstraction over a reader and writer.
///
/// Writes ANSI escape sequences via [`InputParser`] and reads input events.
/// Use [`StdioTerminal`] for local stdio and [`ChannelTerminal`] for SSH/headless.
///
/// Each terminal is one interactive surface (one window), so it owns its own
/// [`Pointer`]: input events carry their source `window`, routing pointer/scroll
/// input to the right surface when many coexist (one per SSH session).
#[derive(Component)]
#[require(crate::prelude::Pointer)]
pub struct Terminal {
	/// Input reader.
	pub reader: Box<dyn 'static + Send + Sync + Read>,
	/// Output writer — receives all ANSI escape sequences and cell data.
	writer: Box<dyn 'static + Send + Sync + Write>,
	/// Configuration applied on init and used to restore previous state on exit.
	config: TerminalConfig,
	input_parser: InputParser,
}

impl Terminal {
	/// Create an in-memory [`Terminal`] backed by a [`Vec<u8>`] writer.
	pub fn new_buffered() -> Self {
		Self::new(Cursor::new(Vec::new()), Vec::new(), default())
	}

	/// Create a terminal with explicit reader, writer, and size.
	pub fn new(
		reader: impl 'static + Send + Sync + Read,
		writer: impl 'static + Send + Sync + Write,
		config: TerminalConfig,
	) -> Self {
		let mut this = Self {
			reader: Box::new(reader),
			writer: Box::new(writer),
			config,
			input_parser: InputParser::new(),
		};
		this.apply_config().unwrap();
		this
	}

	fn apply_config(&mut self) -> Result {
		if self.config.raw_mode {
			// a raw-mode terminal needs a real tty; without one (a sandbox, a
			// headless test, or a piped run) degrade to inert rather than panicking
			// on `tcgetattr` or corrupting output with alt-screen escapes. The host
			// still exists so the scene reduces, it just paints nothing.
			if let Err(err) = raw_mode::enable() {
				warn!("terminal raw mode unavailable, running inert: {err}");
				return Ok(());
			}
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
		if self.config.bracketed_paste {
			self.writer
				.write_all(escape::ENTER_BRACKETED_PASTE.as_bytes())?;
		}
		Ok(())
	}

	pub fn restore_config(&mut self) -> Result {
		Self::restore_config_direct(&self.config, &mut self.writer)
	}

	/// Restore terminal state without initializing the terminal,
	/// used in restore hooks in forced exits like ctrl+c or panic
	pub fn restore_config_direct(
		config: &TerminalConfig,
		writer: &mut impl Write,
	) -> Result {
		if config.bracketed_paste {
			writer.write_all(escape::LEAVE_BRACKETED_PASTE.as_bytes())?;
		}
		if config.enable_mouse {
			writer.write_all(escape::LEAVE_MOUSE.as_bytes())?;
		}
		if config.hide_cursor {
			writer.write_all(escape::SHOW_CURSOR.as_bytes())?;
		}
		if config.alternate_screen {
			writer.write_all(escape::LEAVE_ALT_SCREEN.as_bytes())?;
		}
		if config.raw_mode {
			raw_mode::disable()?;
		}
		Ok(())
	}

	/// Mutable access to the underlying writer.
	pub fn writer_mut(&mut self) -> &mut dyn Write { &mut *self.writer }

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

	/// Reset all SGR attributes.
	pub fn reset_style(&mut self) -> Result {
		self.writer.write_all(escape::RESET.as_bytes())?.xok()
	}

	/// Erase the screen and move the cursor to the home position.
	pub fn clear(&mut self) -> Result {
		self.writer.write_all(escape::ERASE_ALL.as_bytes())?;
		self.writer.write_all(escape::CURSOR_HOME.as_bytes())?.xok()
	}

	/// Draw an iterator of `(pos, cell)` pairs to the terminal.
	///
	/// Style changes are diffed per-cell to minimise escape sequence output.
	/// A single [`escape::RESET`] is emitted at the end of the frame.
	///
	/// Callers are expected to pre-filter cells (eg. via [`DoubleBuffer::diff`])
	/// so that only changed cells are passed.
	fn draw<'a>(
		&mut self,
		cells: impl IntoIterator<Item = (UVec2, &'a super::Cell)>,
	) -> Result {
		// where the terminal cursor lands after the previous write, accounting
		// for wide glyphs advancing two columns.
		let mut cursor: Option<UVec2> = None;
		let mut last_style: Option<VisualStyle> = None;

		for (pos, cell) in cells {
			// a wide glyph's trailing half is drawn by the glyph itself
			if cell.is_wide_continuation() {
				continue;
			}
			// Skip goto when the cursor already sits at the cell.
			if cursor != Some(pos) {
				escape::cursor_goto(&mut self.writer, pos)?;
			}
			cursor = Some(UVec2::new(pos.x + cell.cell_width() as u32, pos.y));

			// Write only the SGR changes since the last written cell.
			cell.style
				.write_style(&mut self.writer, last_style.as_ref())?;
			last_style = Some(cell.style.clone());

			self.writer.write_all(cell.symbol_str().as_bytes())?;
		}

		// Reset style once at the end of the frame.
		if last_style.is_some() {
			self.writer.write_all(escape::RESET.as_bytes())?;
		}
		Ok(())
	}

	pub fn flush(&mut self) -> Result { self.writer.flush()?.xok() }
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

/// Render changed [`DoubleBuffer`]s into their terminal's writer.
pub fn render_terminal(
	mut query: Populated<
		(&mut Terminal, &mut DoubleBuffer),
		Changed<DoubleBuffer>,
	>,
) -> Result {
	for (mut terminal, mut double_buffer) in query.iter_mut() {
		// a resize invalidates the on-screen content (the emulator reflows or
		// pads it), so erase it before redrawing the full frame.
		if double_buffer.take_needs_clear() {
			terminal.clear()?;
		}
		terminal.draw(double_buffer.diff())?;
		double_buffer.swap_buffers();
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

#[cfg(test)]
mod test {
	use super::*;
	use crate::render::charcell::test_host::TestHost;
	use bevy::math::UVec2;

	/// After a resize the renderer erases the screen before redrawing, so stale
	/// glyphs from the emulator-reflowed old frame can't survive wherever the
	/// new frame is blank (the duplicated-letters bug).
	#[beet_core::test]
	fn resize_clears_screen_before_redraw() {
		let mut host = TestHost::sized(UVec2::new(20, 6));
		host.spawn_content(rsx! { <p>"hello"</p> });
		host.step();
		host.frame_ansi(); // drain the boot frame
		// steady state never erases (it would flicker)
		host.step();
		String::from_utf8_lossy(&host.frame_ansi())
			.into_owned()
			.xnot()
			.xpect_contains(escape::ERASE_ALL);
		host.resize(UVec2::new(30, 8));
		host.step();
		let resized = String::from_utf8_lossy(&host.frame_ansi()).into_owned();
		let erase = resized.find(escape::ERASE_ALL);
		erase.is_some().xpect_true();
		// the full frame is redrawn after the erase
		resized[erase.unwrap()..]
			.to_string()
			.xpect_contains("hello");
	}

	/// Drawing a wide glyph advances the cursor two columns, so the cell after
	/// it must not be written one column short (which shifted the rest of the
	/// row right, duplicating letters).
	#[beet_core::test]
	fn wide_glyph_advances_cursor_two_columns() {
		let mut host = TestHost::sized(UVec2::new(10, 2));
		host.spawn_content(rsx! { <pre>"中x"</pre> });
		host.step();
		let out = String::from_utf8_lossy(&host.frame_ansi()).into_owned();
		// the glyph after the wide char is written contiguously, with no goto
		// re-targeting the column the wide char already covered.
		out.as_str().xpect_contains("中x");
		out.xnot().xpect_contains("\u{1b}[1;2H");
	}
}

#[cfg(target_arch = "wasm32")]
mod raw_mode {
	use beet_core::prelude::*;
	pub(super) fn enable() -> Result {
		bevybail!("raw mode not supported on this platform")
	}
	pub(super) fn disable() -> Result {
		bevybail!("raw mode not supported on this platform")
	}
}

/// Unix raw mode support
#[cfg(not(target_arch = "wasm32"))]
mod raw_mode {
	use beet_core::prelude::*;
	use std::io;
	use std::sync::Mutex;
	use std::sync::OnceLock;

	/// Stored original termios, restored on [`disable`].
	static PRIOR_MODE: OnceLock<Mutex<Option<libc::termios>>> = OnceLock::new();

	fn prior_mode() -> &'static Mutex<Option<libc::termios>> {
		PRIOR_MODE.get_or_init(|| Mutex::new(None))
	}

	/// Enable raw mode on stdin (fd 0).
	pub(super) fn enable() -> Result {
		let mut guard = prior_mode().lock().unwrap();
		if guard.is_some() {
			return Ok(()); // already in raw mode
		}
		let fd = libc::STDIN_FILENO;
		// Read current terminal attributes.
		let mut ios: libc::termios = unsafe { core::mem::zeroed() };
		if unsafe { libc::tcgetattr(fd, &mut ios) } != 0 {
			bevybail!("tcgetattr: {}", io::Error::last_os_error());
		}
		let orig = ios;
		// Apply raw mode flags.
		unsafe { libc::cfmakeraw(&mut ios) };
		if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &ios) } != 0 {
			bevybail!("tcsetattr: {}", io::Error::last_os_error());
		}
		*guard = Some(orig);
		Ok(())
	}

	/// Disable raw mode, restoring the original terminal attributes.
	pub(super) fn disable() -> Result {
		let mut guard = prior_mode().lock().unwrap();
		if let Some(orig) = guard.take() {
			let fd = libc::STDIN_FILENO;
			if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &orig) } != 0 {
				bevybail!("tcsetattr: {}", io::Error::last_os_error());
			}
		}
		Ok(())
	}
}
