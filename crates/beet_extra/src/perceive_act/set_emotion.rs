//! `SetEmotion`: the agent sets its face, recorded as an [`Emotion`] component. Mocked
//! in v1/v2 (only records + logs it); the v3 web head renders the matching sprite. The
//! [`Emotion`] + [`SetEmotionInput`] wire types are shared from `perceive_act_core`.
use super::*;
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

#[cfg(test)]
mod test {
	use super::*;
	use crate::perceive_act_core::Emotion;
	use crate::perceive_act_core::SetEmotionInput;
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
