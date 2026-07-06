//! `ShowImage`: the agent displays an image on its face, recorded as a
//! [`DisplayedImage`] component. Mocked in v1/v2 (only records + logs it); the v3
//! web head renders the matching `<img>`. The [`DisplayedImage`] + [`ShowImageInput`]
//! wire types are shared from `perceive_act_core`.
use super::*;
use beet_core::prelude::*;

/// Display an image on your face, chosen to match how you feel about what you see.
///
/// Records the chosen [`DisplayedImage`] (a url) on the caller; read it elsewhere
/// with `Single<&DisplayedImage>`.
#[action(route = "show-image")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn ShowImage(cx: ActionContext<ShowImageInput>) -> Result<()> {
	let src = cx.input.src;
	info!("face: {src}");
	cx.caller.insert(DisplayedImage(src.into())).await?;
	Ok(())
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::perceive_act_core::DisplayedImage;
	use crate::perceive_act_core::ShowImageInput;
	use beet_action::prelude::*;

	#[beet_core::test]
	async fn records_image() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(ShowImage).id();
		world
			.entity_mut(entity)
			.call::<ShowImageInput, ()>(ShowImageInput {
				src: "/assets/extra/perceive-act/explorer/images/joy.png".into(),
			})
			.await
			.unwrap();
		world
			.entity(entity)
			.get::<DisplayedImage>()
			.cloned()
			.xpect_eq(Some(DisplayedImage(
				"/assets/extra/perceive-act/explorer/images/joy.png".into(),
			)));
	}
}
