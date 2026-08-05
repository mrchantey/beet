//! Speech output. v1 shells out to the installed kokoro `tts` command.
use beet_core::prelude::*;

/// Speak `text` aloud via the `tts` command, capturing its output (so it never
/// writes to the terminal and corrupts the TUI) and awaiting until the speech
/// finishes. Errors if `tts` is missing or exits non-zero; the caller decides
/// whether that is fatal. Subprocesses are native-only: a wasm build errors
/// (the web head speaks through `perceive_act_web`'s Web Speech route instead).
pub async fn speak(text: &str) -> Result<()> {
	#[cfg(not(target_arch = "wasm32"))]
	{
		ChildProcess::new("tts")
			.with_args([text])
			.run_async()
			.await
			.map(|_output| ())
	}
	#[cfg(target_arch = "wasm32")]
	{
		bevybail!(
			"`tts` subprocess speech is native-only, cannot speak {text:?} \
			(use the web head's `speak-text` route)"
		)
	}
}
