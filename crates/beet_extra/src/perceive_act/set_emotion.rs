//! `SetEmotion`: the agent sets its face, recorded as an [`Emotion`] component. Mocked
//! in v1 (only records it); v2 renders the matching sprite cell in the TUI.
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
/// `Single<&Emotion>`. v2 binds the TUI face sprite to it.
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
	/// Eager and energetic.
	#[default]
	Excited,
	/// Focused and resolute.
	Determined,
	/// Sad and withdrawn.
	Lonely,
	/// Frustrated and hostile.
	Angry,
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
				emotion: Emotion::Angry,
			})
			.await
			.unwrap();
		world
			.entity(entity)
			.get::<Emotion>()
			.copied()
			.xpect_eq(Some(Emotion::Angry));
	}
}
