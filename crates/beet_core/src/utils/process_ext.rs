use crate::prelude::*;

/// Cross-platform process exit, callable from any thread mid-frame.
/// - Unix: flushes stdout and stderr, then `_exit`s
/// - Other native: `std::process::exit`
/// - Wasm: `js_runtime::exit`
///
/// Unix skips `exit`'s atexit handlers and shared library destructors, which
/// tear a library down beneath any thread still running in it: a GPU driver
/// compiling one of bevy's async pipelines segfaults when a short-lived scene
/// exits mid-compile. Like `std::process::exit`, no Rust destructor runs either
/// way, so the stdio flush is all the teardown an exit performs.
pub fn exit(exit_code: i32) {
	cfg_if! {
		if #[cfg(target_arch = "wasm32")] {
			js_runtime::exit(exit_code);
		} else if #[cfg(unix)] {
			use std::io::Write;
			// a closed pipe has nowhere to flush to, the exit proceeds regardless
			std::io::stdout().flush().ok();
			std::io::stderr().flush().ok();
			// SAFETY: `_exit` takes no pointers and is safe from any thread
			unsafe { libc::_exit(exit_code) }
		} else {
			std::process::exit(exit_code);
		}
	}
}

/// This test binary run again as a child with `args`, ie a test name and a
/// marker flag, for a test that needs a second process.
///
/// Every `BEET_*` variable is scrubbed: a test sets one process-wide (the
/// vault fixture's throwaway `BEET_AGE_IDENTITY`), and the child's runner
/// would otherwise open the nearest `secrets.toml` with it. A variable the
/// child is told by is set on the returned command after.
#[cfg(not(target_arch = "wasm32"))]
pub fn this_test(
	args: impl IntoIterator<Item = impl AsRef<std::ffi::OsStr>>,
) -> Result<std::process::Command> {
	let mut cmd = std::process::Command::new(std::env::current_exe()?);
	for key in env_ext::beet_var_names() {
		cmd.env_remove(key.as_str());
	}
	cmd.args(args);
	cmd.xok()
}

#[cfg(all(test, unix))]
mod test {
	use crate::prelude::*;
	use std::io::Write;

	/// A `BEET_*` variable a test set process-wide never reaches the child,
	/// whose runner would otherwise read it at launch.
	#[crate::test]
	fn this_test_scrubs_beet_vars() {
		const KEY: &str = "BEET_TEST_THIS_TEST_SCRUB";
		// SAFETY: test-only, a name no other test reads
		unsafe { env_ext::set_var(KEY, "leaked") }.unwrap();
		process_ext::this_test(["some_test"])
			.unwrap()
			.get_envs()
			.any(|(key, value)| key == KEY && value.is_none())
			.xpect_true();
	}

	/// Marks the child run of [`exit_skips_library_teardown`].
	const CHILD_FLAG: &str = "--process-ext-exit-child";

	/// Exits from a child run of this test binary: its unterminated stdout line
	/// is flushed, and no atexit handler runs, the teardown that segfaulted a
	/// GPU driver mid-compile when a short-lived scene exited.
	#[crate::test]
	fn exit_skips_library_teardown() {
		if env_ext::args().iter().any(|arg| arg == CHILD_FLAG) {
			extern "C" fn on_exit() {
				let marker = b"ATEXIT";
				// SAFETY: writes a static buffer to the stdout fd
				unsafe { libc::write(1, marker.as_ptr().cast(), marker.len()) };
			}
			// SAFETY: registers a plain `extern "C"` fn
			unsafe { libc::atexit(on_exit) };
			std::io::stdout().write_all(b"answer").unwrap();
			process_ext::exit(3);
		}
		let output = process_ext::this_test([
			"exit_skips_library_teardown",
			"--quiet",
			CHILD_FLAG,
		])
		.unwrap()
		.output()
		.unwrap();
		output.status.code().xpect_eq(Some(3));
		String::from_utf8_lossy(&output.stdout)
			.xpect_contains("answer")
			.xnot()
			.xpect_contains("ATEXIT");
	}
}
