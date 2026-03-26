use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_node::prelude::*;
use beet_tool::prelude::*;

/// Creates a render tool that negotiates content type via the
/// `Accept` header and delegates to [`MediaRenderer`].
pub async fn default_scene_renderer(
	cx: AsyncToolIn<RequestParts>,
) -> Result<Response> {
	let accepts: Vec<MediaType> = cx
		.input
		.headers
		.get::<header::Accept>()
		.and_then(|result| result.ok())
		.unwrap_or_default();

	cx.caller
		.with_then(move |entity: EntityWorldMut| -> Result<Response> {
			let id = entity.id();
			let world = entity.into_world_mut();

			let mut cx = RenderContext::new(id, world).with_accepts(accepts);
			let output = MediaRenderer::default().render(&mut cx)?;

			match output {
				RenderOutput::Media(bytes) => {
					Response::ok_body(bytes.bytes(), bytes.media_type().clone())
				}
				RenderOutput::Stateful => {
					Response::ok_body("state updated.", MediaType::Text)
				}
			}
			.xok()
		})
		.await
}
