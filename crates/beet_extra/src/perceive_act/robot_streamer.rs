//! The perceive-act agent's model streamer, switchable to a mock for socket testing.
//!
//! [`RobotStreamer`] replaces the inline `<ModelStreamer provider="OpenAi"/>` on the
//! `Robot` actor so the one shared `agent.bsx` runs either the real model or a mock with
//! no scene fork. With the [`MOCK_MODEL_ENV`] env var set, the model is stubbed with a
//! canned loop of [`RespondMultiModalInput`] calls ([`robot_mock_payloads`]) instead of
//! calling OpenAI, so the socket plumbing (browser head + device body) can be hardened
//! without paying to interpret an image every cycle. The mock still drives the full
//! `set-emotion`/`speak-text`/`drive` fan-out over the sockets, just with hardcoded
//! values.
use super::*;
use crate::beet::prelude::*;
use beet_core::prelude::*;

/// When this env var is set, [`RobotStreamer`] stubs the model with [`MockPostStreamer`]
/// (a canned `respond-multi-modal` loop) instead of calling OpenAI, so socket testing
/// makes no network calls.
pub const MOCK_MODEL_ENV: &str = "BEET_MOCK_MODEL";

/// The `Robot` actor's model streamer: the real OpenAI streamer, or a mock cycling
/// canned `respond-multi-modal` calls when [`MOCK_MODEL_ENV`] is set.
///
/// The real path mirrors `<ModelStreamer provider="OpenAi"/>` (gpt-5.4-mini, the given
/// reasoning `effort`); the mock path exercises the same fan-out with lively hardcoded
/// values and no network call. One switch point keeps `agent.bsx` shared across v1/v2/v3.
#[template]
pub fn RobotStreamer(
	/// Reasoning effort for the real model (eg `ReasoningEffort::None` for the
	/// fastest reply); ignored by the mock.
	#[prop(default)]
	effort: Option<o11s::ReasoningEffort>,
) -> impl Bundle {
	OnSpawn::new(move |entity: &mut EntityWorldMut| -> Result {
		if env_ext::var(MOCK_MODEL_ENV).is_ok() {
			info!("perceive-act: mocking the model ({MOCK_MODEL_ENV} set)");
			entity.insert(MockPostStreamer::with_tool_arguments(
				robot_mock_payloads()?,
			));
		} else {
			entity.insert_template(ModelStreamer {
				provider: Provider::OpenAi,
				// optional template props are `PropOpt`-wrapped in a struct literal.
				effort: beet_core::types::PropOpt(effort),
				..default()
			})?;
		}
		Ok(())
	})
}

/// A short hand-authored loop of `respond-multi-modal` payloads the mock cycles: a
/// gentle wander with swinging emotions and short lines, so a no-network run still
/// exercises the face, speech and drive fan-out and looks alive. Velocities are gentle
/// and steps sub-2s (the agent also clamps to `max_drive_duration`), so the body stays
/// tame.
fn robot_mock_payloads() -> Result<Vec<String>> {
	// (image title, line, linear mm/s, angular deg/s (+left), seconds); unknown
	// titles fall back to the scene's fallback image.
	[
		("joy", "Ooh, shiny floor!", 60., 0., 1.2),
		("confused", "What's over here?", 40., 30., 1.0),
		("excited", "Zoom zoom!", 80., 0., 1.0),
		("surprised", "Whoa, careful!", 20., -60., 1.2),
		("calm", "Just cruising along.", 50., 0., 1.5),
		("sad", "Aw, a dead end.", 15., 90., 1.0),
	]
	.into_iter()
	.map(|(image, say, linear, angular, secs)| {
		serde_json::to_string(&RespondMultiModalInput {
			image: image.into(),
			say: say.into(),
			drive: DriveForDuration {
				drive: DifferentialDrive::new(linear, angular),
				duration: Duration::from_secs_f32(secs),
			},
			parallel: false,
		})
		.map_err(Into::into)
	})
	.collect()
}
