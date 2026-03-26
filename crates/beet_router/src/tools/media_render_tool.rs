//! Render tool that selects a renderer based on the request's
//! `Accept` header via [`MediaRenderer`].
//!
//! Falls back to HTML when no `Accept` header is present.
//!
//! # Usage
//!
//! ```ignore
//! use beet_router::prelude::*;
//!
//! commands.spawn((my_server(), children![media_render_tool()]));
//! ```
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_node::prelude::*;
use beet_tool::prelude::*;

/// Creates a render tool that negotiates content type via the
/// `Accept` header and delegates to [`MediaRenderer`].
///
/// On each request it:
/// 1. Reads the `Accept` header from the original request
/// 2. Renders the spawned content entity in that format
/// 3. Despawns the content entity
/// 4. Returns the rendered content as a [`Response`]
pub fn media_render_tool() -> impl Bundle {
	(
		Name::new("Media Render Tool"),
		RenderToolMarker,
		RouteHidden,
		async_tool(
			async |cx: AsyncToolIn<RenderRequest>| -> Result<Response> {
				let spawn_tool = cx.input.spawn_tool.clone();
				let world = cx.caller.world();

				let accepts: Vec<MediaType> = cx
					.input
					.request
					.headers
					.get::<header::Accept>()
					.and_then(|result| result.ok())
					.unwrap_or_default();

				// Spawn the scene content on demand
				let entity = cx.caller.call_detached(spawn_tool, ()).await?;

				// Render in the resolved format, then despawn
				world
					.with_then(move |world: &mut World| -> Result<Response> {
						let mut cx = RenderContext::new(entity, world)
							.with_accepts(accepts);
						let output =
							MediaRenderer::default().render(&mut cx)?;
						world.entity_mut(entity).despawn();

						match output {
							RenderOutput::Media(bytes) => Response::ok_body(
								bytes.bytes(),
								bytes.media_type().clone(),
							),
							RenderOutput::Stateful => Response::ok_body(
								"state updated.",
								MediaType::Text,
							),
						}
						.xok()
					})
					.await
			},
		),
	)
}
