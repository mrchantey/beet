//! `ShowImage`: the `show-image` capability, shared by the native mock head and the
//! wasm browser head. It records the chosen [`DisplayedImage`] url on the caller; the
//! web head additionally renders it into `<img id="face">` (its `render_face` observer),
//! the native mock just logs it. Lives here (not the `thread`-gated agent module) so
//! both heads share one definition.
use super::*;
use beet_core::prelude::*;

/// Display an image on your face, chosen to match how you feel about what you see.
///
/// Records the chosen [`DisplayedImage`] (a url) on the caller; read it elsewhere with
/// `Single<&DisplayedImage>`.
#[action(route = "show-image")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn ShowImage(cx: ActionContext<ShowImageInput>) -> Result<()> {
	let src = cx.input.src;
	info!("show-image: {src}");
	cx.caller.insert(DisplayedImage(src.into())).await?;
	Ok(())
}

#[cfg(test)]
mod test {
	use super::*;
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
