//! `InterpretPhoto`: the agent's "look and tell me what you see" tool, `In = ()`,
//! `Out = String`. It captures a photo and one-shots it to a vision model for a
//! description. Distinct from [`TakePhoto`](super::TakePhoto), the raw capture: this
//! is the agent-facing tool that adds the describe.
use crate::beet::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

/// Look at what is in front of you and get back a description of it.
///
/// Captures via the `take-photo` route on the agent's own router (rather than a direct
/// local capture), so a bound head client serves the capture while the describe stays
/// here on the agent (the brief's "client captures, agent describes"). With no head
/// bound, `take-photo`'s local handler captures the floor-photo fixtures instead.
#[action(route = "interpret-photo")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn InterpretPhoto(cx: ActionContext<InterpretPhotoInput>) -> Result<String> {
	let media = cx
		.caller
		.call_detached(route_action(), Request::get("take-photo"))
		.await?
		.into_result()
		.await?
		.into_media_bytes()
		.await?;
	describe_image(media).await
}

/// No arguments. An empty struct rather than `()` so the tool schema is a JSON object
/// (`{}`), which OpenAI function-calling requires and which the model's empty `{}`
/// arguments deserialize into (a `()` input rejects the map).
#[derive(Reflect, serde::Deserialize, serde::Serialize)]
pub struct InterpretPhotoInput {}

/// One-shot the photo to a vision model and return its description. Swap the streamer
/// line to change provider (the agent itself is set in the scene's `{ModelStreamer}`).
async fn describe_image(media: MediaBytes) -> Result<String> {
	run_oneshot(children![
		(
			Actor::user(),
			children![
				Post::spawn(
					"You are the eyes of a small floor robot. In one or two sentences, \
					describe anything of interest in front of you, and any obstacle worth avoiding."
				),
				Post::spawn(IntoPost::Bytes {
					media_type: media.media_type().clone(),
					bytes: media.bytes().to_vec(),
					file_stem: None,
				}),
			]
		),
		(Actor::agent(), OpenAiProvider::gpt_5_mini()?),
	])
	.await?
	.into_iter()
	.filter(|post| post.intent().is_display())
	.map(|post| post.to_string())
	.collect::<String>()
	.xok()
}
