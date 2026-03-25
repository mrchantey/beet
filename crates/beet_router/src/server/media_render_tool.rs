//! Render tool that selects a renderer based on the request's
//! `Accept` header via [`MediaRenderer`].
//!
//! [`media_render_tool`] reads the [`Accept`](header::Accept) header
//! and dispatches to the appropriate renderer:
//!
//! 1. `text/html` → HTML
//! 2. `text/markdown` → Markdown
//! 3. `text/plain` → Markdown (readable fallback)
//! 4. `application/json` → JSON scene (feature-gated)
//! 5. `application/x-postcard` → Postcard scene (feature-gated)
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
/// 2. Picks the best supported media type
/// 3. Renders the spawned content entity in that format
/// 4. Despawns the content entity
/// 5. Returns the rendered content as a [`Response`]
pub fn media_render_tool() -> impl Bundle {
	(
		Name::new("Media Render Tool"),
		RenderToolMarker,
		RouteHidden,
		async_tool(
			async |cx: AsyncToolIn<RenderRequest>| -> Result<Response> {
				let spawn_tool = cx.input.spawn_tool.clone();
				let world = cx.caller.world();

				let accepted: Vec<MediaType> = cx
					.input
					.request
					.headers
					.get::<header::Accept>()
					.and_then(|result| result.ok())
					.unwrap_or_default();

				let target = resolve_media_type(&accepted);

				// Spawn the scene content on demand
				let entity = cx.caller.call_detached(spawn_tool, ()).await?;

				// Render in the resolved format, then despawn
				let response = world
					.with_then(move |world: &mut World| -> Result<Response> {
						let response = match &target {
							MediaType::Json => render_json(world, entity),
							MediaType::Postcard => {
								render_postcard(world, entity)
							}
							media_type => {
								render_media(world, entity, media_type.clone())
							}
						}?;
						world.entity_mut(entity).despawn();
						response.xok()
					})
					.await?;

				response.xok()
			},
		),
	)
}

/// Resolve the target media type from the accepted types list.
/// Falls back to HTML when no preference is expressed or
/// no supported type is found.
fn resolve_media_type(accepted: &[MediaType]) -> MediaType {
	if accepted.is_empty() {
		return MediaType::Html;
	}
	for media_type in accepted {
		match media_type {
			MediaType::Html
			| MediaType::Markdown
			| MediaType::Json
			| MediaType::Postcard
			| MediaType::AnsiTerm => return media_type.clone(),
			MediaType::Text => return MediaType::Markdown,
			MediaType::Bytes => return MediaType::Postcard,
			_ => continue,
		}
	}
	MediaType::Html
}

/// Render via [`MediaRenderer`] for text-based formats.
fn render_media(
	world: &mut World,
	entity: Entity,
	media_type: MediaType,
) -> Result<Response> {
	let mut render_cx = RenderContext::new(entity, world)
		.with_accepts(vec![media_type.clone()]);
	let output = MediaRenderer::new(media_type.clone())
		.render(&mut render_cx)
		.map_err(|err| bevyhow!("{err}"))?;
	let media_bytes = match output {
		RenderOutput::Media(bytes) => bytes,
		RenderOutput::Stateful => {
			bevybail!("unexpected stateful render")
		}
	};
	let body = String::from_utf8_lossy(media_bytes.bytes()).into_owned();
	Response::ok_body(body, media_type).xok()
}

/// Serialize the entity tree as JSON via [`SceneSaver`].
#[cfg(all(feature = "bevy_scene", feature = "json"))]
fn render_json(world: &mut World, entity: Entity) -> Result<Response> {
	let bytes = SceneSaver::new(world)
		.with_entity_tree(entity)
		.save_json()?;
	Response::ok_body(bytes, MediaType::Json).xok()
}

#[cfg(not(all(feature = "bevy_scene", feature = "json")))]
fn render_json(_world: &mut World, _entity: Entity) -> Result<Response> {
	bevybail!("JSON scene format requires the `bevy_scene` and `json` features")
}

/// Serialize the entity tree as postcard via [`SceneSaver`].
#[cfg(all(feature = "bevy_scene", feature = "postcard"))]
fn render_postcard(world: &mut World, entity: Entity) -> Result<Response> {
	let bytes = SceneSaver::new(world)
		.with_entity_tree(entity)
		.save_postcard()?;
	Response::ok_body(bytes, MediaType::Postcard).xok()
}

#[cfg(not(all(feature = "bevy_scene", feature = "postcard")))]
fn render_postcard(_world: &mut World, _entity: Entity) -> Result<Response> {
	bevybail!(
		"Postcard scene format requires the `bevy_scene` and `postcard` features"
	)
}
