//! Render semantic text content to markdown format.
//!
//! This module provides functionality to convert the semantic text representation
//! (using [`TextNode`] and semantic markers) into markdown strings.
//!
//! Supports all content types produced by [`MarkdownDiffer`](crate::parsers::MarkdownDiffer):
//!
//! - Block elements: headings, paragraphs, block quotes, code blocks,
//!   lists, tables, thematic breaks, images
//! - Inline markers: strong, emphasis, strikethrough, code, quote,
//!   superscript, subscript, links
//!
//! The core rendering logic lives in [`MarkdownRenderer`] which implements
//! [`CardVisitor`]. This module wires it up as a tool and provides
//! [`render_markdown_for`] for direct world access.
//! ```
use crate::prelude::*;
use beet_core::prelude::*;


/// Creates a markdown rendering tool for an entity's text content tree.
///
/// This tool traverses the entity and its descendants within the card
/// boundary, converting semantic markers to their markdown equivalents:
///
/// - [`Heading1`]..=[`Heading6`] → `#`..=`######` (heading level)
/// - [`Paragraph`] → `text\n\n` (paragraph with trailing newlines)
/// - [`Important`] → `**text**` (bold)
/// - [`Emphasize`] → `*text*` (italic)
/// - [`Code`] → `` `text` `` (inline code)
/// - [`Quote`] → `"text"` (quoted)
/// - [`Link`] → `[text](url)`
/// - [`BlockQuote`] → `> text` (block quote)
/// - [`CodeBlock`] → fenced code block
/// - [`ListMarker`] + [`ListItem`] → `- item` or `1. item`
/// - [`ThematicBreak`] → `---`
/// - [`Image`] → `![alt](src)`
/// - [`Strikethrough`] → `~~text~~`
/// - [`Table`] → GFM table
/// - [`Superscript`] → `^text^`
/// - [`Subscript`] → `~text~`
///
/// # Returns
///
/// A bundle containing the markdown rendering tool that produces a markdown
/// string representing the text content of the entity tree.
///
/// # Example
///
/// ```ignore
/// use beet_stack::prelude::*;
/// use beet_core::prelude::*;
///
/// let val = World::new()
/// 			.spawn((render_markdown(), Paragraph::with_text("hello world")))
/// 			.send_blocking::<(), String>(())
/// 			.unwrap();
/// assert_eq!(val, "hello world");
///
pub fn render_markdown() -> impl Bundle {
	(
		PathPartial::new("render-markdown"),
		tool(render_markdown_system),
	)
}

/// Renders an entity's text content tree to markdown using direct world access.
///
/// This is the reusable entry point for markdown rendering. It runs the
/// rendering system via [`World::run_system_cached_with`], so it can be
/// called from any context that has `&mut World`.
pub fn render_markdown_for(entity: Entity, world: &mut World) -> String {
	world
		.run_system_cached_with(render_markdown_for_entity, entity)
		.unwrap_or_default()
}

/// System that renders an entity tree to markdown using [`CardWalker`].
/// Used by the [`render_markdown`] tool, which renders relative to
/// its own entity (via card root resolution).
fn render_markdown_system(
	In(cx): In<ToolContext>,
	walker: CardWalker,
) -> Result<String> {
	let mut renderer = MarkdownRenderer::new();
	walker.walk_card(&mut renderer, cx.tool);
	renderer.finish().xok()
}

/// System that renders a specific entity to markdown, starting from
/// that entity directly rather than resolving the card root first.
/// Used by [`render_markdown_for`].
fn render_markdown_for_entity(
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

	#[test]
	fn plain_text() {
		World::new()
			.spawn((render_markdown(), children![TextNode::new("hello world")]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("hello world");
	}

	#[test]
	fn multiple_segments() {
		World::new()
			.spawn((render_markdown(), children![
				TextNode::new("hello"),
				TextNode::new(" "),
				TextNode::new("world"),
			]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("hello world");
	}

	#[test]
	fn important_text() {
		World::new()
			.spawn((render_markdown(), children![
				TextNode::new("hello "),
				(Important, children![TextNode::new("bold")]),
				TextNode::new(" text"),
			]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("hello **bold** text");
	}

	#[test]
	fn emphasized_text() {
		World::new()
			.spawn((render_markdown(), children![
				TextNode::new("hello "),
				(Emphasize, children![TextNode::new("italic")]),
				TextNode::new(" text"),
			]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("hello *italic* text");
	}

	#[test]
	fn code_text() {
		World::new()
			.spawn((render_markdown(), children![
				TextNode::new("use "),
				(Code, children![TextNode::new("println!")]),
				TextNode::new(" macro"),
			]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("use `println!` macro");
	}

	#[test]
	fn quoted_text() {
		World::new()
			.spawn((render_markdown(), children![
				TextNode::new("he said "),
				(Quote, children![TextNode::new("hello")]),
			]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("he said \"hello\"");
	}

	#[test]
	fn link_without_title() {
		World::new()
			.spawn((render_markdown(), children![(
				Link::new("https://example.com"),
				children![TextNode::new("click here")],
			)]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("[click here](https://example.com)");
	}

	#[test]
	fn link_with_title() {
		World::new()
			.spawn((render_markdown(), children![(
				Link::new("https://example.com").with_title("Example Site"),
				children![TextNode::new("example")],
			)]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("[example](https://example.com \"Example Site\")");
	}


	#[test]
	fn combined_markers() {
		World::new()
			.spawn((render_markdown(), children![(Important, children![(
				Emphasize,
				children![TextNode::new("bold italic")],
			)],)]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("***bold italic***");
	}

	#[test]
	fn complex_composition() {
		World::new()
			.spawn((render_markdown(), children![
				TextNode::new("Welcome to "),
				(Important, children![TextNode::new("beet")]),
				TextNode::new(", the "),
				(Emphasize, children![TextNode::new("best")]),
				TextNode::new(" framework!"),
			]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("Welcome to **beet**, the *best* framework!");
	}

	#[test]
	fn extension_trait() {
		World::new()
			.spawn((render_markdown(), children![TextNode::new("test")]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("test");
	}

	#[test]
	fn important_link() {
		World::new()
			.spawn((render_markdown(), children![(Important, children![(
				Link::new("https://example.com"),
				children![TextNode::new("important link")],
			)],)]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("[**important link**](https://example.com)");
	}

	#[test]
	fn all_markers_combined() {
		World::new()
			.spawn((render_markdown(), children![(Quote, children![(
				Important,
				children![(Emphasize, children![(Code, children![(
					Link::new("https://example.com"),
					children![TextNode::new("text")],
				)],)],)],
			)],)]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("[\"***`text`***\"](https://example.com)");
	}

	#[test]
	fn heading_renders_as_heading() {
		World::new()
			.spawn((render_markdown(), Heading1::with_text("Hello World")))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("# Hello World\n\n");
	}

	#[test]
	fn heading2_renders_as_h2() {
		World::new()
			.spawn((render_markdown(), children![
				Heading1::with_text("Outer"),
				Heading2::with_text("Inner"),
			]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("# Outer\n\n## Inner\n\n");
	}

	#[test]
	fn paragraph_renders_with_newlines() {
		World::new()
			.spawn((
				render_markdown(),
				Paragraph::with_text("A paragraph of text."),
			))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("A paragraph of text.\n\n");
	}

	#[test]
	fn mixed_structure() {
		World::new()
			.spawn((render_markdown(), children![
				Heading1::with_text("Welcome"),
				Paragraph::with_text("This is the intro.")
			]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("# Welcome\n\nThis is the intro.\n\n");
	}

	#[test]
	fn respects_card_boundary() {
		World::new()
			.spawn((render_markdown(), Card, children![
				Paragraph::with_text("Inside card"),
				// Nested card should not be rendered
				(Card, children![Paragraph::with_text("Inside nested card")])
			]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("Inside card\n\n");
	}

	#[test]
	fn render_markdown_for_works() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![
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
			.spawn((Card, children![
				Paragraph::with_text("visible"),
				(Card, children![Paragraph::with_text("hidden")])
			]))
			.id();

		let result = render_markdown_for(entity, &mut world);
		result.xpect_eq("visible\n\n");
	}
}
