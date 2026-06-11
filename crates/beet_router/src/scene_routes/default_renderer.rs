use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// Creates a render action that negotiates content type via the
/// `Accept` header and delegates to [`MediaRenderer`].
pub async fn default_renderer(
	cx: ActionContext<RequestParts>,
) -> Result<Response> {
	let accepts: Vec<MediaType> = cx
		.input
		.headers
		.get::<header::Accept>()
		.and_then(|result| result.ok())
		.unwrap_or_default();

	cx.caller
		.with(move |entity: EntityWorldMut| -> Result<Response> {
			let id = entity.id();
			let world = entity.into_world_mut();

			let mut cx = RenderContext::new(id, world).with_accepts(accepts);
			let bytes = MediaRenderer::default().render(&mut cx)?;
			Response::ok().with_media(bytes).xok()
		})
		.await
		.flatten()
}
