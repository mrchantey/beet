//! The perceive-act tool wire types shared by the agent and every head client.
//!
//! Pure reflect + serde types, so the JSON the agent forwards over the socket
//! deserializes identically whether the head that serves it is the native mock, the
//! wgpu body, or the wasm browser head. No `beet_thread`, so they build in the wasm
//! `web` head as readily as the native agent.
use beet_core::prelude::*;

/// What to say out loud, the input to the `speak-text` capability.
#[derive(Reflect, serde::Deserialize, serde::Serialize)]
pub struct SpeakTextInput {
	/// The line to say out loud, in character. Keep it short and full of personality.
	pub text: String,
}

/// What expression to wear, the input to the `set-emotion` capability.
#[derive(Reflect, serde::Deserialize, serde::Serialize)]
pub struct SetEmotionInput {
	/// The expression to show on the face.
	pub emotion: Emotion,
}

/// The expression currently shown on the face, set by the `set-emotion` capability and
/// read with `Single<&Emotion>`.
///
/// Each variant names one of the eight robot-eyes sprites in
/// `assets/extra/robot-eyes/` ([`sprite_stem`](Emotion::sprite_stem) is the file
/// stem), so the web head renders the current face as a pure lookup.
#[derive(
	Debug,
	Default,
	Clone,
	Copy,
	PartialEq,
	Component,
	Reflect,
	serde::Deserialize,
	serde::Serialize,
)]
#[reflect(Component, Default)]
pub enum Emotion {
	/// Frustrated and hostile.
	Anger,
	/// At ease, the neutral resting face.
	#[default]
	Calm,
	/// Puzzled by something unexpected.
	Confused,
	/// Repulsed by something unpleasant.
	Disgust,
	/// Eager and energetic.
	Excited,
	/// Delighted and happy.
	Joy,
	/// Downcast and withdrawn.
	Sad,
	/// Taken aback by something sudden.
	Surprised,
}

impl Emotion {
	/// The file stem of this expression's sprite in `assets/extra/robot-eyes/`, eg
	/// `Emotion::Joy` -> `"joy"` (`joy.png`). The lowercased variant name, so the
	/// enum and the sprite set stay in lockstep.
	pub fn sprite_stem(&self) -> &'static str {
		match self {
			Emotion::Anger => "anger",
			Emotion::Calm => "calm",
			Emotion::Confused => "confused",
			Emotion::Disgust => "disgust",
			Emotion::Excited => "excited",
			Emotion::Joy => "joy",
			Emotion::Sad => "sad",
			Emotion::Surprised => "surprised",
		}
	}
}
