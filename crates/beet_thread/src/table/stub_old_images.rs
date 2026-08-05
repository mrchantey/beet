use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Bound the thread window's image bytes, as a standalone action.
///
/// The action form of [`ThreadWindow::stub_old_images`]: every image post older
/// than the most recent `keep` is replaced in place by a short text stub, so an
/// endless loop's request stays bounded without ever dropping a post. Sequence it
/// after whatever posts the image:
///
/// ```rsx
/// <div {Thread} {Sequence}>
///     <CreateActor name="Camera" kind="User" {PostPhoto}/>
///     <Template {StubOldImages}/>
/// </div>
/// ```
///
/// Standing on its own entity is the point: bounding is a policy of the thread,
/// not of whichever actor happened to capture the image, so a thread whose images
/// arrive from a tool call or an upload bounds them the same way.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(Action<(), Outcome> = Action::new_async(stub_old_images_action))]
pub struct StubOldImages {
	/// How many recent images keep their bytes; older ones are stubbed to text.
	pub keep: usize,
}

impl Default for StubOldImages {
	fn default() -> Self {
		// the current + previous image: images dominate request bytes, and two are
		// enough to see what changed since the last turn.
		Self { keep: 2 }
	}
}

/// Stub this entity's thread window down to its configured image count.
async fn stub_old_images_action(cx: ActionContext) -> Result<Outcome> {
	let keep = cx.caller.get_cloned::<StubOldImages>().await?.keep;
	cx.caller
		.with_state::<ThreadWindowQuery, _>(
			move |entity, mut windows| -> Result {
				windows.window_mut(entity)?.stub_old_images(keep);
				Ok(())
			},
		)
		.await??;
	Ok(Pass(()))
}
