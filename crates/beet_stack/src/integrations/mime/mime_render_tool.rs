//! Content-negotiation render tool that selects a renderer based on
//! the request's `Accept` header.
//!
//! [`mime_render_tool`] inspects the [`Accept`] header from the
//! original request and dispatches to the best available renderer:
//!
//! 1. `text/html` → [`HtmlRenderer`]
//! 2. `text/markdown` → [`MarkdownRenderer`] (requires `markdown` feature)
//! 3. `text/plain` → [`HtmlRenderer`] (readable fallback)
//! 4. `application/json` → scene serialized as JSON via [`SceneSaver`]
//! 5. `application/x-postcard` → scene serialized as postcard via [`SceneSaver`]
//!
//! If no `Accept` header is present, or none of the requested types
//! are supported, it falls back to HTML.
//!
//! # Usage
//!
//! Place this on a server entity instead of a format-specific tool:
//!
//! ```ignore
//! use beet_stack::prelude::*;
//!
//! // Instead of markdown_render_tool() or html_render_tool():
//! commands.spawn((my_server(), children![mime_render_tool()]));
//! ```
use crate::prelude::*;
use beet_core::prelude::*;

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

				// Spawn the card content on demand
				let card_entity = cx.caller.call_detached(spawn_tool, ()).await?;

				// Render in the negotiated format, then despawn
				let response = world
					.with_then(move |world: &mut World| -> Result<Response> {
						let response = match accept {
							NegotiatedFormat::Html => {
								let html = render_html_for(card_entity, world);
								Response::ok_body(html, MimeType::Html)
							}
							#[cfg(feature = "markdown")]
							NegotiatedFormat::Markdown => {
								let md =
									render_markdown_for(card_entity, world);
								Response::ok_body(md, MimeType::Markdown)
							}
							#[cfg(not(feature = "markdown"))]
							NegotiatedFormat::Markdown => {
								// Fall back to HTML when markdown is unavailable
								let html = render_html_for(card_entity, world);
								Response::ok_body(html, MimeType::Html)
							}
							#[cfg(all(feature = "bevy_scene", feature = "json"))]
							NegotiatedFormat::Json => render_scene_json(card_entity, world)?,
							#[cfg(not(all(feature = "bevy_scene", feature = "json")))]
							NegotiatedFormat::Json => {
								bevybail!(
									"JSON scene format requires the `bevy_scene` and `json` features"
								);
							}
							#[cfg(all(feature = "bevy_scene", feature = "postcard"))]
							NegotiatedFormat::Postcard => render_scene_postcard(card_entity, world)?,
							#[cfg(not(all(feature = "bevy_scene", feature = "postcard")))]
							NegotiatedFormat::Postcard => {
								bevybail!(
									"Postcard scene format requires the `bevy_scene` and `postcard` features"
								);
							}
						};
						world.entity_mut(card_entity).despawn();
						response.xok()
					})
					.await?;

				response.xok()
			},
		),
	)
}

/// Serialize an entity tree to a JSON scene [`Response`].
#[cfg(all(feature = "bevy_scene", feature = "json"))]
fn render_scene_json(entity: Entity, world: &mut World) -> Result<Response> {
	let bytes = SceneSaver::new(world)
		.with_entity_tree(entity)
		.save_json()?;
	Response::ok_body(bytes, MimeType::Json).xok()
}

/// Serialize an entity tree to a postcard scene [`Response`].
#[cfg(all(feature = "bevy_scene", feature = "postcard"))]
fn render_scene_postcard(
	entity: Entity,
	world: &mut World,
) -> Result<Response> {
	let bytes = SceneSaver::new(world)
		.with_entity_tree(entity)
		.save_postcard()?;
	Response::ok_body(bytes, MimeType::Postcard).xok()
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
	let accepted: Vec<MimeType> = request
		.headers
		.get::<header::Accept>()
		.and_then(|result| result.ok())
		.unwrap_or_default();

	// No Accept header → default to HTML
	if accepted.is_empty() {
		return NegotiatedFormat::Html;
	}

	// Walk the quality-sorted list and pick the first we support
	for mime in &accepted {
		match mime {
			MimeType::Html => return NegotiatedFormat::Html,
			MimeType::Markdown => return NegotiatedFormat::Markdown,
			MimeType::Text => return NegotiatedFormat::Markdown,
			MimeType::Json => return NegotiatedFormat::Json,
			MimeType::Postcard | MimeType::Bytes => {
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

	// -- negotiate_format --

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
