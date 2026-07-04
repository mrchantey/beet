//! `SpeakText`: the agent's spoken, in-character voice. The [`SpeakTextInput`] wire type
//! is shared from `perceive_act_core`.
use super::*;
use beet_core::prelude::*;

/// Say something out loud, in character. This is your spoken voice, heard by whoever
/// is nearby, distinct from your private train of thought.
#[action(route = "speak-text")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn SpeakText(cx: ActionContext<SpeakTextInput>) -> Result<()> {
	let line = cx.input.text;
	info!("speak: \"{line}\"");
	// speech is best-effort: a missing or failing `tts` must not break the loop.
	if let Err(err) = speech_ext::speak(&line).await {
		warn!("could not speak: {err}");
	}
	Ok(())
}
