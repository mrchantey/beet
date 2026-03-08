//! HTML tools for rendering card content and entity trees.
//!
//! This module provides two tools and a convenience function:
//!
//! - [`html_render_tool`]: A [`RenderToolMarker`] tool placed on
//!   servers. It handles [`RenderRequest`] by rendering the spawned
//!   content entity via [`HtmlRenderer`] and despawning it.
//!
//! - [`render_html`]: A standalone tool that renders an entity's own
//!   text content tree to an HTML string.
//!
//! - [`render_html_for`]: Direct world access entry point for
//!   rendering a specific entity to HTML.
//!
//! The core rendering logic lives in [`HtmlRenderer`] which
//! implements [`CardVisitor`].
use crate::prelude::*;
use beet_core::prelude::*;

/// Creates a render tool that renders cards to HTML.
///
/// Used by HTTP servers for browser output. Should be placed on the
/// server entity, not the router — different servers need different
/// render tools.
///
/// On each request it:
/// 1. Renders the spawned content entity to HTML
/// 2. Despawns the content entity
/// 3. Returns the HTML as a [`Response`]
pub fn html_render_tool() -> impl Bundle {
	(
		Name::new("HTML Render Tool"),
		RenderToolMarker,
		RouteHidden,
		async_tool(
			async |cx: AsyncToolIn<RenderRequest>| -> Result<Response> {
				let spawn_tool = cx.input.spawn_tool.clone();
				let world = cx.caller.world();

				// Spawn the card content on demand
				let card_entity =
					cx.caller.call_detached(spawn_tool, ()).await?;

				// Render to HTML, then despawn
				let html = world
					.with_then(move |world: &mut World| -> String {
						let html = render_html_for(card_entity, world);
						world.entity_mut(card_entity).despawn();
						html
					})
					.await;

				Response::ok_body(html, MediaType::Html).xok()
			},
		),
	)
}

/// Creates a standalone HTML rendering tool for an entity's text
/// content tree.
///
/// Traverses the entity and its descendants within the card
/// boundary, converting semantic markers to their HTML equivalents.
/// See [`HtmlRenderer`] for the full list of supported elements.
///
/// # Example
///
/// ```
/// use beet_stack::prelude::*;
/// use beet_core::prelude::*;
///
/// AsyncPlugin::world()
///     .spawn((render_html(), Paragraph::with_text("hello world")))
///     .call_blocking::<(), String>(())
///     .unwrap()
///     .xpect_eq("<p>hello world</p>");
/// ```
pub fn render_html() -> impl Bundle {
	(
		PathPartial::new("render-html"),
		system_tool(render_html_system),
	)
}

/// Renders an entity's text content tree to HTML using direct world
/// access.
///
/// Runs the rendering system via [`World::run_system_cached_with`],
/// so it can be called from any context with `&mut World`.
pub fn render_html_for(entity: Entity, world: &mut World) -> String {
	world
		.run_system_cached_with(render_html_for_system, entity)
		.unwrap_or_default()
}

/// System that renders an entity tree to HTML using [`CardWalker`].
/// Renders relative to the tool's own entity via card root resolution.
fn render_html_system(
	In(cx): In<SystemToolIn>,
	walker: CardWalker,
) -> Result<String> {
	let mut renderer = HtmlRenderer::new();
	walker.walk_card(&mut renderer, cx.caller);
	renderer.finish().xok()
}

/// System that renders a specific entity to HTML, starting from that
/// entity directly rather than resolving the card root first.
fn render_html_for_system(
	In(entity): In<Entity>,
	walker: CardWalker,
) -> String {
	let mut renderer = HtmlRenderer::new();
	walker.walk_from(&mut renderer, entity);
	renderer.finish()
}


#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	async fn plain_text() {
		AsyncPlugin::world()
			.spawn((render_html(), children![TextNode::new("hello world")]))
			.call::<(), String>(())
			.await
			.unwrap()
			.xpect_eq("hello world");
	}

	#[beet_core::test]
	async fn multiple_segments() {
		AsyncPlugin::world()
			.spawn((render_html(), children![
				TextNode::new("hello"),
				TextNode::new(" "),
				TextNode::new("world"),
			]))
			.call::<(), String>(())
			.await
			.unwrap()
			.xpect_eq("hello world");
	}

	#[beet_core::test]
	async fn important_text() {
		AsyncPlugin::world()
			.spawn((render_html(), children![
				TextNode::new("hello "),
				(Important, children![TextNode::new("bold")]),
				TextNode::new(" text"),
			]))
			.call::<(), String>(())
			.await
			.unwrap()
			.xpect_eq("hello <strong>bold</strong> text");
	}

	#[beet_core::test]
	async fn emphasized_text() {
		AsyncPlugin::world()
			.spawn((render_html(), children![
				TextNode::new("hello "),
				(Emphasize, children![TextNode::new("italic")]),
				TextNode::new(" text"),
			]))
			.call::<(), String>(())
			.await
			.unwrap()
			.xpect_eq("hello <em>italic</em> text");
	}

	#[beet_core::test]
	async fn heading_renders() {
		AsyncPlugin::world()
			.spawn((render_html(), Heading1::with_text("Hello World")))
			.call::<(), String>(())
			.await
			.unwrap()
			.xpect_eq("<h1>Hello World</h1>");
	}

	#[beet_core::test]
	async fn paragraph_renders() {
		AsyncPlugin::world()
			.spawn((
				render_html(),
				Paragraph::with_text("A paragraph of text."),
			))
			.call::<(), String>(())
			.await
			.unwrap()
			.xpect_eq("<p>A paragraph of text.</p>");
	}

	#[beet_core::test]
	async fn mixed_structure() {
		AsyncPlugin::world()
			.spawn((render_html(), children![
				Heading1::with_text("Welcome"),
				Paragraph::with_text("This is the intro.")
			]))
			.call::<(), String>(())
			.await
			.unwrap()
			.xpect_eq("<h1>Welcome</h1><p>This is the intro.</p>");
	}

	#[beet_core::test]
	async fn respects_card_boundary() {
		AsyncPlugin::world()
			.spawn((render_html(), CardTool, children![
				Paragraph::with_text("Inside card"),
				(CardTool, children![Paragraph::with_text(
					"Inside nested card"
				)])
			]))
			.call::<(), String>(())
			.await
			.unwrap()
			.xpect_eq("<p>Inside card</p>");
	}

	#[beet_core::test]
	async fn link_without_title() {
		AsyncPlugin::world()
			.spawn((render_html(), children![
				Link::new("https://example.com").with_text("click here"),
			]))
			.call::<(), String>(())
			.await
			.unwrap()
			.xpect_eq("<a href=\"https://example.com\">click here</a>");
	}

	#[test]
	fn render_html_for_works() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![
				TextNode::new("hello "),
				(Important, children![TextNode::new("world")]),
			]))
			.id();

		let result = render_html_for(entity, &mut world);
		result.xpect_eq("hello <strong>world</strong>");
	}

	#[test]
	fn render_html_for_respects_card_boundary() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![
				Paragraph::with_text("visible"),
				(CardTool, children![Paragraph::with_text("hidden")])
			]))
			.id();

		let result = render_html_for(entity, &mut world);
		result.xpect_eq("<p>visible</p>");
	}
}
