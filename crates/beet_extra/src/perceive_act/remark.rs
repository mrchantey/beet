//! `Remark`: the agent's spoken, in-character voice.
use super::speech_ext;
use beet_core::prelude::*;

/// Say something out loud, in character. This is your spoken voice, heard by whoever
/// is nearby, distinct from your private train of thought.
#[action(route = "remark")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn Remark(cx: ActionContext<RemarkInput>) -> Result<()> {
	let line = cx.input.text;
	info!("Remark: {line}");
	// speech is best-effort: a missing or failing `tts` must not break the loop.
	if let Err(err) = speech_ext::speak(&line).await {
		warn!("could not speak remark: {err}");
	}
	Ok(())
}

/// What to say out loud.
#[derive(Reflect, serde::Deserialize, serde::Serialize)]
pub struct RemarkInput {
	/// The line to say out loud, in character. Keep it short and full of personality.
	pub text: String,
}
