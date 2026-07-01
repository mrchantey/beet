//! `SetEmotion`: the agent sets its face, recorded as an [`Emotion`] component. Mocked
//! in v1/v2 (only records + logs it); the v3 web head renders the matching sprite.
use beet_core::prelude::*;

/// Set your facial expression to match how you feel about what is happening.
///
/// Records the chosen [`Emotion`] on the caller; read it elsewhere with
/// `Single<&Emotion>`.
#[action(route = "set-emotion")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn SetEmotion(cx: ActionContext<SetEmotionInput>) -> Result<()> {
	let emotion = cx.input.emotion;
	info!("SetEmotion: {emotion:?}");
	cx.caller.insert(emotion).await?;
	Ok(())
}

/// What expression to wear.
#[derive(Reflect, serde::Deserialize, serde::Serialize)]
pub struct SetEmotionInput {
	/// The expression to show on the face.
	pub emotion: Emotion,
}

/// The expression currently shown on the face, set by [`SetEmotion`] and read with
/// `Single<&Emotion>`.
///
/// Each variant names one of the eight robot-eyes sprites in
/// `assets/extra/robot-eyes/` ([`sprite_stem`](Emotion::sprite_stem) is the file
/// stem), so the v3 web head renders the current face as a pure lookup.
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

#[cfg(test)]
mod test {
	use super::*;
	use beet_action::prelude::*;

	#[beet_core::test]
	async fn records_emotion() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(SetEmotion).id();
		world
			.entity_mut(entity)
			.call::<SetEmotionInput, ()>(SetEmotionInput {
				emotion: Emotion::Anger,
			})
			.await
			.unwrap();
		world
			.entity(entity)
			.get::<Emotion>()
			.copied()
			.xpect_eq(Some(Emotion::Anger));
	}
}
