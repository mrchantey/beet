//! `SpeakText`: the browser head's spoken voice, `In = SpeakTextInput`, `Out = ()`.
//!
//! Serves the same `speak-text` route the desktop head does, but through the Web
//! Speech API instead of a `tts` subprocess: `speechSynthesis.speak(new
//! SpeechSynthesisUtterance(text))`. Best-effort, matching the mock: a missing or
//! blocked synthesizer must never break the agent loop.
use super::*;
use beet_core::prelude::*;
use web_sys::SpeechSynthesisUtterance;

/// Say `text` out loud via the browser's speech synthesizer. Best-effort: any failure
/// is logged, not surfaced, so a headless or muted tab still drives the loop.
#[action(route = "speak-text")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn SpeakText(cx: ActionContext<SpeakTextInput>) -> Result<()> {
	let line = cx.input.text;
	info!("SpeakText: {line}");
	if let Err(err) = speak(&line) {
		warn!("could not speak: {err:?}");
	}
	Ok(())
}

/// Queue `text` on `window.speechSynthesis`. Returns immediately, letting the browser
/// speak asynchronously, so the socket handler never blocks on the audio finishing.
fn speak(text: &str) -> Result<()> {
	let synthesis = web_sys::window()
		.ok_or_else(|| bevyhow!("no window"))?
		.speech_synthesis()
		.map_jserr()?;
	let utterance = SpeechSynthesisUtterance::new_with_text(text).map_jserr()?;
	synthesis.speak(&utterance);
	Ok(())
}
