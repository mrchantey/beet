//! Speech output. v1 shells out to the installed kokoro `tts` command.
use beet_core::prelude::*;

/// Speak `text` aloud via the `tts` command, capturing its output (so it never
/// writes to the terminal and corrupts the TUI) and awaiting until the speech
/// finishes. Errors if `tts` is missing or exits non-zero; the caller decides
/// whether that is fatal.
pub async fn speak(text: &str) -> Result<()> {
	ChildProcess::new("tts")
		.with_args([text])
		.run_async()
		.await
		.map(|_output| ())
}
