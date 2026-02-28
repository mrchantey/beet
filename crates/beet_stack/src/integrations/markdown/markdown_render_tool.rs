//! Markdown tools for rendering card content and entity trees.
//!
//! This module provides two tools and a convenience function:
//!
//! - [`markdown_render_tool`]: A [`RenderToolMarker`] tool placed on
//!   CLI/REPL servers. It handles [`RenderRequest`] by rendering the
//!   spawned content entity via [`MarkdownRenderer`] and despawning it.
//!
//! - [`render_markdown`]: A standalone tool that renders an entity's
//!   own text content tree to a markdown string.
//!
//! - [`render_markdown_for`]: Direct world access entry point for
//!   rendering a specific entity to markdown.
//!
//! The core rendering logic lives in [`MarkdownRenderer`] which
//! implements [`CardVisitor`].
use crate::prelude::*;
use beet_core::prelude::*;

/// Creates a render tool that renders cards to markdown.
///
/// Used by CLI and REPL servers for terminal output. Should be
/// placed on the server entity, not the router — different servers
/// need different render tools.
///
/// On each request it:
/// 1. Renders the spawned content entity to markdown
/// 2. Despawns the content entity
/// 3. Returns the markdown as a [`Response`]
pub fn markdown_render_tool() -> impl Bundle {
	(
		Name::new("Markdown Render Tool"),
		RenderToolMarker,
		RouteHidden,
		async_tool(
			async |cx: AsyncToolIn<RenderRequest>| -> Result<Response> {
				let spawn_tool = cx.input.spawn_tool.clone();
				let world = cx.tool.world();

				// Spawn the card content on demand
				let card_entity = cx.tool.call_tool(spawn_tool, ()).await?;

				// Render to markdown, then despawn
				let markdown = world
					.with_then(move |world: &mut World| -> String {
						let markdown = render_markdown_for(card_entity, world);
						world.entity_mut(card_entity).despawn();
						markdown
					})
					.await;

				Response::ok_body(markdown, MimeType::Text).xok()
			},
		),
	)
}

/// Creates a standalone markdown rendering tool for an entity's
/// text content tree.
///
/// Traverses the entity and its descendants within the card
/// boundary, converting semantic markers to their markdown
/// equivalents. See [`MarkdownRenderer`] for the full list of
/// supported elements.
///
/// # Example
///
/// ```
/// use beet_stack::prelude::*;
/// use beet_core::prelude::*;
///
/// AsyncPlugin::world()
///     .spawn((render_markdown(), Paragraph::with_text("hello world")))
///     .call_blocking::<(), String>(())
///     .unwrap()
///     .xpect_eq("hello world\n\n");
/// ```
pub fn render_markdown() -> impl Bundle {
	(
		PathPartial::new("render-markdown"),
		system_tool(render_markdown_system),
	)
}

/// Renders an entity's text content tree to markdown using direct
/// world access.
///
/// Runs the rendering system via [`World::run_system_cached_with`],
/// so it can be called from any context with `&mut World`.
pub fn render_markdown_for(entity: Entity, world: &mut World) -> String {
	world
		.run_system_cached_with(render_markdown_for_system, entity)
		.unwrap_or_default()
}

/// System that renders an entity tree to markdown using [`CardWalker`].
/// Renders relative to the tool's own entity via card root resolution.
fn render_markdown_system(
	In(cx): In<SystemToolIn>,
	walker: CardWalker,
) -> Result<String> {
	let mut renderer = MarkdownRenderer::new();
	walker.walk_card(&mut renderer, cx.tool);
	renderer.finish().xok()
}

/// System that renders a specific entity to markdown, starting from
/// that entity directly rather than resolving the card root first.
fn render_markdown_for_system(
	In(entity): In<Entity>,
	walker: CardWalker,
) -> String {
	let mut renderer = MarkdownRenderer::new();
	walker.walk_from(&mut renderer, entity);
	renderer.finish()
}


#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	async fn plain_text() {
		AsyncPlugin::world()
			.spawn((render_markdown(), children![TextNode::new("hello world")]))
			.call::<(), String>(())
			.await
			.unwrap()
			.xpect_eq("hello world");
	}

	#[beet_core::test]
	async fn multiple_segments() {
		AsyncPlugin::world()
			.spawn((render_markdown(), children![
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
			.spawn((render_markdown(), children![
				TextNode::new("hello "),
				(Important, children![TextNode::new("bold")]),
				TextNode::new(" text"),
			]))
			.call::<(), String>(())
			.await
			.unwrap()
			.xpect_eq("hello **bold** text");
	}

	#[beet_core::test]
	async fn emphasized_text() {
		AsyncPlugin::world()
			.spawn((render_markdown(), children![
				TextNode::new("hello "),
				(Emphasize, children![TextNode::new("italic")]),
				TextNode::new(" text"),
			]))
			.call::<(), String>(())
			.await
			.unwrap()
			.xpect_eq("hello *italic* text");
	}

	#[beet_core::test]
	async fn code_text() {
		AsyncPlugin::world()
			.spawn((render_markdown(), children![
				TextNode::new("use "),
				(Code, children![TextNode::new("println!")]),
				TextNode::new(" macro"),
			]))
			.call::<(), String>(())
			.await
			.unwrap()
			.xpect_eq("use `println!` macro");
	}

	#[beet_core::test]
	async fn quoted_text() {
		AsyncPlugin::world()
			.spawn((render_markdown(), children![
				TextNode::new("he said "),
				(Quote, children![TextNode::new("hello")]),
			]))
			.call::<(), String>(())
			.await
			.unwrap()
			.xpect_eq("he said \"hello\"");
	}

	#[beet_core::test]
	async fn link_without_title() {
		AsyncPlugin::world()
			.spawn((render_markdown(), children![
				Link::new("https://example.com").with_text("click here"),
			]))
			.call::<(), String>(())
			.await
			.unwrap()
			.xpect_eq("[click here](https://example.com)");
	}

	#[beet_core::test]
	async fn link_with_title() {
		AsyncPlugin::world()
			.spawn((render_markdown(), children![
				Link::new("https://example.com")
					.with_title("Example Site")
					.with_text("example"),
			]))
			.call::<(), String>(())
			.await
			.unwrap()
			.xpect_eq("[example](https://example.com \"Example Site\")");
	}

	#[beet_core::test]
	async fn combined_markers() {
		AsyncPlugin::world()
			.spawn((render_markdown(), children![(Important, children![(
				Emphasize,
				children![TextNode::new("bold italic")],
			)],)]))
			.call::<(), String>(())
			.await
			.unwrap()
			.xpect_eq("***bold italic***");
	}

	#[beet_core::test]
	async fn complex_composition() {
		AsyncPlugin::world()
			.spawn((render_markdown(), children![
				TextNode::new("Welcome to "),
				(Important, children![TextNode::new("beet")]),
				TextNode::new(", the "),
				(Emphasize, children![TextNode::new("best")]),
				TextNode::new(" framework!"),
			]))
			.call::<(), String>(())
			.await
			.unwrap()
			.xpect_eq("Welcome to **beet**, the *best* framework!");
	}

	#[beet_core::test]
	async fn important_link() {
		AsyncPlugin::world()
			.spawn((render_markdown(), children![(Important, children![
				Link::new("https://example.com").with_text("important link"),
			])]))
			.call::<(), String>(())
			.await
			.unwrap()
			.xpect_eq("[**important link**](https://example.com)");
	}

	#[beet_core::test]
	async fn all_markers_combined() {
		AsyncPlugin::world()
			.spawn((render_markdown(), children![(Quote, children![(
				Important,
				children![(Emphasize, children![(Code, children![
					Link::new("https://example.com").with_text("text"),
				])],)],
			)],)]))
			.call::<(), String>(())
			.await
			.unwrap()
			.xpect_eq("[\"***`text`***\"](https://example.com)");
	}

	#[beet_core::test]
	async fn heading_renders() {
		AsyncPlugin::world()
			.spawn((render_markdown(), Heading1::with_text("Hello World")))
			.call::<(), String>(())
			.await
			.unwrap()
			.xpect_eq("# Hello World\n\n");
	}

	#[beet_core::test]
	async fn heading2_renders() {
		AsyncPlugin::world()
			.spawn((render_markdown(), children![
				Heading1::with_text("Outer"),
				Heading2::with_text("Inner"),
			]))
			.call::<(), String>(())
			.await
			.unwrap()
			.xpect_eq("# Outer\n\n## Inner\n\n");
	}

	#[beet_core::test]
	async fn paragraph_renders_with_newlines() {
		AsyncPlugin::world()
			.spawn((
				render_markdown(),
				Paragraph::with_text("A paragraph of text."),
			))
			.call::<(), String>(())
			.await
			.unwrap()
			.xpect_eq("A paragraph of text.\n\n");
	}

	#[beet_core::test]
	async fn mixed_structure() {
		AsyncPlugin::world()
			.spawn((render_markdown(), children![
				Heading1::with_text("Welcome"),
				Paragraph::with_text("This is the intro.")
			]))
			.call::<(), String>(())
			.await
			.unwrap()
			.xpect_eq("# Welcome\n\nThis is the intro.\n\n");
	}

	#[beet_core::test]
	async fn respects_card_boundary() {
		AsyncPlugin::world()
			.spawn((render_markdown(), CardTool, children![
				Paragraph::with_text("Inside card"),
				// Nested card should not be rendered
				(CardTool, children![Paragraph::with_text(
					"Inside nested card"
				)])
			]))
			.call::<(), String>(())
			.await
			.unwrap()
			.xpect_eq("Inside card\n\n");
	}

	#[test]
	fn render_markdown_for_works() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![
				TextNode::new("hello "),
				(Important, children![TextNode::new("world")]),
			]))
			.id();

		let result = render_markdown_for(entity, &mut world);
		result.xpect_eq("hello **world**");
	}

	#[test]
	fn render_markdown_for_respects_card_boundary() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![
				Paragraph::with_text("visible"),
				(CardTool, children![Paragraph::with_text("hidden")])
			]))
			.id();

		let result = render_markdown_for(entity, &mut world);
		result.xpect_eq("visible\n\n");
	}
}
