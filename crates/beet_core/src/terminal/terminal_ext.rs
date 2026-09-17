//! Unix terminal utilities
#[allow(unused)]
use crate::prelude::*;
use std::io::Write;
use std::io::stdout;
#[allow(unused)]
use std::path::Path;

/// Open the controlling terminal `/dev/tty` for writing.
///
/// The live TUI renders its frames here rather than to `stdout`, leaving
/// `stdout`/`stderr` free for logging (see [`redirect_std_to_file`]). Input is
/// already read from `/dev/tty` for the same separation.
#[cfg(not(target_arch = "wasm32"))]
pub fn tty_writer() -> std::io::Result<std::fs::File> {
	std::fs::OpenOptions::new().write(true).open("/dev/tty")
}

/// wasm has no controlling terminal; callers fall back to `stdout`.
#[cfg(target_arch = "wasm32")]
pub fn tty_writer() -> std::io::Result<std::fs::File> {
	Err(std::io::Error::new(
		std::io::ErrorKind::Unsupported,
		"no /dev/tty on wasm",
	))
}

/// Redirect this process's `stdout` and `stderr` to `path`, creating any
/// missing parent directories and appending to the file.
///
/// Done at the file-descriptor level (`dup2`), so it captures everything written
/// to fd 1/2: tracing logs, stray `println!`/`eprintln!`, and panic messages
/// (whose default hook writes to `stderr`). The live TUI uses this with a
/// `/dev/tty` frame writer so diagnostics never corrupt the alternate screen.
#[cfg(all(not(target_arch = "wasm32"), feature = "crossterm"))]
pub fn redirect_std_to_file(path: impl AsRef<Path>) -> Result {
	use std::os::unix::io::AsRawFd;
	let path = path.as_ref();
	if let Some(parent) = path.parent() {
		fs_ext::create_dir_all(parent)?;
	}
	let file = std::fs::OpenOptions::new()
		.create(true)
		.append(true)
		.open(path)?;
	let fd = file.as_raw_fd();
	// point fd 1 (stdout) and fd 2 (stderr) at the log file. The dup'd fds
	// outlive `file`, so leak it to keep the underlying description open.
	if unsafe { libc::dup2(fd, libc::STDOUT_FILENO) } < 0 {
		bevybail!("dup2 stdout: {}", std::io::Error::last_os_error());
	}
	if unsafe { libc::dup2(fd, libc::STDERR_FILENO) } < 0 {
		bevybail!("dup2 stderr: {}", std::io::Error::last_os_error());
	}
	core::mem::forget(file);
	Ok(())
}

/// wasm cannot redirect file descriptors; callers fall back to `stdout`.
#[cfg(target_arch = "wasm32")]
pub fn redirect_std_to_file(_path: impl AsRef<Path>) -> Result {
	bevybail!("redirecting std is not supported on wasm")
}

/// Prompt on the controlling terminal and read one line with echo off, the
/// way a passphrase is typed: the prompt and a newline after the entry go to
/// the tty, the line never reaches stdout or the scrollback. Errors when
/// there is no tty (a pipe, a headless runner), so a caller can say what to
/// pass instead.
#[cfg(all(unix, feature = "secrets"))]
pub fn read_secret_line(prompt: &str) -> Result<String> {
	use std::io::BufRead;
	use std::os::unix::io::AsRawFd;
	let mut tty = tty_writer().map_err(|err| {
		bevyhow!(
			"no terminal to prompt on ({err}): a secret is typed, not piped"
		)
	})?;
	tty.write_all(prompt.as_bytes())?;
	tty.flush()?;
	let input = std::fs::File::open("/dev/tty")?;
	let fd = input.as_raw_fd();
	// SAFETY: a zeroed termios is a valid out-param for tcgetattr, and the
	// restored attributes are the ones just read.
	let mut original: libc::termios = unsafe { core::mem::zeroed() };
	if unsafe { libc::tcgetattr(fd, &mut original) } != 0 {
		bevybail!("tcgetattr: {}", std::io::Error::last_os_error());
	}
	let mut quiet = original;
	quiet.c_lflag &= !libc::ECHO;
	if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &quiet) } != 0 {
		bevybail!("tcsetattr: {}", std::io::Error::last_os_error());
	}
	let mut line = String::new();
	let read = std::io::BufReader::new(&input).read_line(&mut line);
	// echo comes back whatever the read did
	unsafe { libc::tcsetattr(fd, libc::TCSANOW, &original) };
	tty.write_all(b"\n")?;
	read?;
	line.trim_end_matches(['\r', '\n']).to_string().xok()
}

/// A platform with no controlling terminal to read from.
#[cfg(all(not(unix), feature = "secrets"))]
pub fn read_secret_line(_prompt: &str) -> Result<String> {
	bevybail!("no terminal to prompt on: a secret is typed, not piped")
}

/// Adds this handler to both the panic and ctrl+c hooks
pub fn on_force_exit(
	func: impl 'static + Send + Sync + Clone + Fn(),
) -> Result {
	#[cfg(all(not(target_arch = "wasm32"), feature = "ctrlc"))]
	{
		let func2 = func.clone();
		ctrlc::set_handler(move || {
			func2();
			std::process::exit(0);
		})?;
	}
	// update_hook when stablizes
	let prev = std::panic::take_hook();
	std::panic::set_hook(Box::new(move |info| {
		func();
		prev(info);
	}));
	Ok(())
}

/// Shows the terminal cursor.
///
/// ```
/// # use beet_core::prelude::*;
/// terminal_ext::show_cursor();
/// ```
pub fn show_cursor() {
	let mut stdout = stdout();
	stdout.write_all(b"\x1B[?25h").unwrap();
}

/// Hides the terminal cursor.
///
/// ```
/// # use beet_core::prelude::*;
/// terminal_ext::hide_cursor();
/// ```
pub fn hide_cursor() {
	let mut stdout = stdout();
	stdout.write_all(b"\x1B[?25l").unwrap();
}

/// Resets the cursor to the home position (0, 0).
///
/// # Examples
///
/// ```no_run
/// # use beet_core::prelude::*;
/// terminal_ext::reset_cursor();
/// ```
pub fn reset_cursor() { move_to(0, 0).ok(); }

/// Clears the terminal screen and moves the cursor to the home position.
///
/// # Examples
///
/// ```no_run
/// # use beet_core::prelude::*;
/// terminal_ext::clear().unwrap();
/// ```
///
/// # Errors
///
/// Returns an error if the write or flush operations fail.
pub fn clear() -> std::io::Result<()> {
	let mut stdout = stdout();
	stdout.write_all(b"\x1B[2J")?;
	stdout.write_all(b"\x1B[H")?;
	stdout.flush()?;
	Ok(())
}

/// Moves the cursor to the specified position (x, y).
///
/// # Arguments
///
/// * `x` - The column position (0-indexed)
/// * `y` - The row position (0-indexed)
///
/// # Examples
///
/// ```no_run
/// # use beet_core::prelude::*;
/// terminal_ext::move_to(10, 5).unwrap();
/// ```
///
/// # Errors
///
/// Returns an error if the write or flush operations fail.
pub fn move_to(x: u16, y: u16) -> std::io::Result<()> {
	let mut stdout = stdout();
	stdout.write_all(format!("\x1B[{};{}H", y + 1, x + 1).as_bytes())?;
	stdout.flush()?;
	Ok(())
}

/// Returns the terminal size, defaulting to 80,24 if it could not be determined.
pub fn size() -> UVec2 {
	let default_size = UVec2::new(80, 24);
	cfg_if! {
		if #[cfg(all(not(target_arch = "wasm32"), feature = "crossterm"))]{
			crossterm::terminal::size()
				.map(|(cols,rows)|UVec2::new(cols as u32, rows as u32))
				.unwrap_or(default_size)
		}else {
			default_size
		}
	}
}
