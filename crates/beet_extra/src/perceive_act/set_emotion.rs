//! `SetEmotion`: the agent sets its face. Mocked in v1 (records the emotion); v2
//! renders the matching sprite cell in the TUI.
use beet_core::prelude::*;

/// Set your facial expression to match how you feel about what is happening.
#[action(route = "set-emotion")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn SetEmotion(cx: ActionContext<SetEmotionInput>) -> Result<()> {
	let emotion = cx.input.emotion;
	info!("SetEmotion (mock): {emotion:?}");
	cx.caller
		.with_state::<ResMut<CurrentEmotion>, _>(move |_entity, mut current| {
			current.0 = emotion;
		})
		.await?;
	Ok(())
}

/// A facial expression the agent can wear.
#[derive(
	Debug,
	Default,
	Clone,
	Copy,
	PartialEq,
	Reflect,
	serde::Deserialize,
	serde::Serialize,
)]
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

/// What expression to wear.
#[derive(Reflect, serde::Deserialize, serde::Serialize)]
pub struct SetEmotionInput {
	/// The expression to show on the face.
	pub emotion: Emotion,
}

/// The emotion currently shown on the face, set by [`SetEmotion`]. v2 binds the TUI
/// face sprite to this.
#[derive(Debug, Default, Clone, Resource)]
pub struct CurrentEmotion(pub Emotion);

#[cfg(test)]
mod test {
	use super::*;
	use beet_action::prelude::*;

	#[beet_core::test]
	async fn records_emotion() {
		let mut world = AsyncPlugin::world();
		world.insert_resource(CurrentEmotion::default());
		let entity = world.spawn(SetEmotion).id();
		world
			.entity_mut(entity)
			.call::<SetEmotionInput, ()>(SetEmotionInput {
				emotion: Emotion::Angry,
			})
			.await
			.unwrap();
		world.resource::<CurrentEmotion>().0.xpect_eq(Emotion::Angry);
	}
}
