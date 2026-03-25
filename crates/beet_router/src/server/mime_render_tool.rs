//! Content-negotiation render tool that selects a renderer based on
//! the request's `Accept` header.
//!
//! [`mime_render_tool`] inspects the [`Accept`](header::Accept) header
//! from the original request and dispatches to the best available
//! renderer via [`MediaRenderer`]:
//!
//! 1. `text/html` → HTML
//! 2. `text/markdown` → Markdown
//! 3. `text/plain` → Markdown (readable fallback)
//! 4. `application/json` → JSON scene (feature-gated stub)
//! 5. `application/x-postcard` → Postcard scene (feature-gated stub)
//!
//! If no `Accept` header is present, or none of the requested types
//! are supported, it falls back to HTML.
//!
//! # Usage
//!
//! Place this on a server entity instead of a format-specific tool:
//!
//! ```ignore
//! use beet_router::prelude::*;
//!
//! // Instead of a format-specific render tool:
//! commands.spawn((my_server(), children![mime_render_tool()]));
//! ```
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_node::prelude::*;
use beet_tool::prelude::*;

/// Creates a render tool that negotiates content type via the
/// `Accept` header.
///
/// Prefers HTML, then markdown, then JSON, then postcard, falling
/// back to HTML when the client doesn't specify a preference.
///
/// On each request it:
/// 1. Reads the `Accept` header from the original request
/// 2. Picks the best supported format
/// 3. Renders the spawned content entity in that format
/// 4. Despawns the content entity
/// 5. Returns the rendered content as a [`Response`]
pub fn mime_render_tool() -> impl Bundle {
	(
		Name::new("MIME Render Tool"),
		RenderToolMarker,
		RouteHidden,
		async_tool(
			async |cx: AsyncToolIn<RenderRequest>| -> Result<Response> {
				let spawn_tool = cx.input.spawn_tool.clone();
				let world = cx.caller.world();

				let accept = negotiate_format(&cx.input.request);

				let target_media_type = match accept {
					NegotiatedFormat::Html => MediaType::Html,
					NegotiatedFormat::Markdown => MediaType::Markdown,
					NegotiatedFormat::Json => MediaType::Json,
					NegotiatedFormat::Postcard => MediaType::Postcard,
				};

				// Spawn the scene content on demand
				let entity = cx.caller.call_detached(spawn_tool, ()).await?;

				// Render in the negotiated format, then despawn
				let response = world
					.with_then(move |world: &mut World| -> Result<Response> {
						let response = match accept {
							NegotiatedFormat::Html
							| NegotiatedFormat::Markdown => {
								let mut render_cx =
									RenderContext::new(entity, world)
										.with_accepts(vec![
											target_media_type.clone(),
										]);
								let output =
									MediaRenderer::new(
										target_media_type.clone(),
									)
									.render(&mut render_cx)
									.map_err(|err| bevyhow!("{err}"))?;
								let media_bytes = match output {
									RenderOutput::Media(bytes) => bytes,
									RenderOutput::Stateful => {
										bevybail!(
											"unexpected stateful render"
										)
									}
								};
								let body = String::from_utf8_lossy(
									media_bytes.bytes(),
								)
								.into_owned();
								world.entity_mut(entity).despawn();
								Response::ok_body(body, target_media_type)
							}
							#[cfg(all(
								feature = "bevy_scene",
								feature = "json"
							))]
							NegotiatedFormat::Json => {
								let bytes = SceneSaver::new(world)
									.with_entity_tree(entity)
									.save_json()?;
								world.entity_mut(entity).despawn();
								Response::ok_body(bytes, MediaType::Json)
							}
							#[cfg(not(all(
								feature = "bevy_scene",
								feature = "json"
							)))]
							NegotiatedFormat::Json => {
								world.entity_mut(entity).despawn();
								bevybail!(
									"JSON scene format requires the `bevy_scene` and `json` features"
								);
							}
							#[cfg(all(
								feature = "bevy_scene",
								feature = "postcard"
							))]
							NegotiatedFormat::Postcard => {
								let bytes = SceneSaver::new(world)
									.with_entity_tree(entity)
									.save_postcard()?;
								world.entity_mut(entity).despawn();
								Response::ok_body(bytes, MediaType::Postcard)
							}
							#[cfg(not(all(
								feature = "bevy_scene",
								feature = "postcard"
							)))]
							NegotiatedFormat::Postcard => {
								world.entity_mut(entity).despawn();
								bevybail!(
									"Postcard scene format requires the `bevy_scene` and `postcard` features"
								);
							}
						};
						response.xok()
					})
					.await?;

				response.xok()
			},
		),
	)
}

/// The format chosen by content negotiation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NegotiatedFormat {
	Html,
	Markdown,
	Json,
	Postcard,
}

/// Inspect the request's `Accept` header and pick the best supported
/// format. Falls back to HTML when no preference is expressed.
fn negotiate_format(request: &Request) -> NegotiatedFormat {
	let accepted: Vec<MediaType> = request
		.headers
		.get::<header::Accept>()
		.and_then(|result| result.ok())
		.unwrap_or_default();

	// No Accept header → default to HTML
	if accepted.is_empty() {
		return NegotiatedFormat::Html;
	}

	// Walk the quality-sorted list and pick the first we support
	for media_type in &accepted {
		match media_type {
			MediaType::Html => return NegotiatedFormat::Html,
			MediaType::Markdown => return NegotiatedFormat::Markdown,
			MediaType::Text => return NegotiatedFormat::Markdown,
			MediaType::Json => return NegotiatedFormat::Json,
			MediaType::Postcard | MediaType::Bytes => {
				return NegotiatedFormat::Postcard;
			}
			_ => continue,
		}
	}

	// None of the requested types are supported — default to HTML
	NegotiatedFormat::Html
}


#[cfg(test)]
mod test {
	use super::*;

	/// Helper to build a request with a raw Accept header.
	fn request_with_accept(accept: &str) -> Request {
		let mut request = Request::get("/");
		request.headers.set_raw("accept", accept);
		request
	}

	#[test]
	fn defaults_to_html_without_accept() {
		let request = Request::get("/");
		negotiate_format(&request).xpect_eq(NegotiatedFormat::Html);
	}

	#[test]
	fn prefers_html() {
		let request = request_with_accept("text/html");
		negotiate_format(&request).xpect_eq(NegotiatedFormat::Html);
	}

	#[test]
	fn selects_markdown() {
		let request = request_with_accept("text/markdown");
		negotiate_format(&request).xpect_eq(NegotiatedFormat::Markdown);
	}

	#[test]
	fn text_plain_falls_back_to_markdown() {
		let request = request_with_accept("text/plain");
		negotiate_format(&request).xpect_eq(NegotiatedFormat::Markdown);
	}

	#[test]
	fn quality_ordering_prefers_higher() {
		// markdown has higher quality than html
		let request =
			request_with_accept("text/html;q=0.5, text/markdown;q=0.9");
		negotiate_format(&request).xpect_eq(NegotiatedFormat::Markdown);
	}

	#[test]
	fn selects_json() {
		let request = request_with_accept("application/json");
		negotiate_format(&request).xpect_eq(NegotiatedFormat::Json);
	}

	#[test]
	fn selects_postcard() {
		let request = request_with_accept("application/x-postcard");
		negotiate_format(&request).xpect_eq(NegotiatedFormat::Postcard);
	}

	#[test]
	fn bytes_selects_postcard() {
		let request = request_with_accept("application/octet-stream");
		negotiate_format(&request).xpect_eq(NegotiatedFormat::Postcard);
	}

	#[test]
	fn unsupported_type_falls_back_to_html() {
		let request = request_with_accept("application/xml");
		negotiate_format(&request).xpect_eq(NegotiatedFormat::Html);
	}
}
