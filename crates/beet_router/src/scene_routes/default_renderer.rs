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

			// always render HTML through the reactive renderer in `Auto` mode: a
			// page with bindings gets the thin-client wire format + the runtime
			// (loaded from the shared, cached `/js/reactivity.js`), while a plain
			// page emits no blob and no script, so the static output is unchanged.
			let mut renderer = MediaRenderer::default()
				.with_html_renderer(HtmlRenderer::new().reactive());

			let mut cx = RenderContext::new(id, world).with_accepts(accepts);
			let bytes = renderer.render(&mut cx)?;
			Response::ok().with_media(bytes).xok()
		})
		.await
		.flatten()
}
