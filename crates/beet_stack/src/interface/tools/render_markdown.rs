//! Render semantic text content to markdown format.
//!
//! This module provides functionality to convert the semantic text representation
//! (using [`TextContent`] and semantic markers) into markdown strings.
//!
//! The core rendering logic is exposed via [`render_markdown_for`] so it can
//! be called from other systems (ie the interface tool) without needing
//! to go through a tool call.
//! ```

use crate::prelude::*;
use beet_core::prelude::*;


/// Routes a request to the matching card or tool, rendering cards as markdown.
///
/// Looks up the request path in the [`RouteTree`], then either renders
/// the card's content tree as markdown or forwards the request to a
/// tool via `entity.call`.
pub(crate) async fn route_handler(
	cx: AsyncToolContext<Request>,
) -> Result<Outcome<Response, Request>> {
	let path = cx.input.path().clone();
	let tool_entity = cx.tool.id();
	let world = cx.tool.world();

	let node = world
		.with_then(move |world: &mut World| -> Option<RouteNode> {
			let tree = root_route_tree(world, tool_entity).ok()?;
			tree.find(&path).cloned()
		})
		.await;

	match node {
		Some(RouteNode::Card(card_node)) => {
			let card_entity = card_node.entity;
			let markdown = world
				.with_then(move |world: &mut World| {
					render_markdown_for(card_entity, world)
				})
				.await;
			Pass(Response::ok_body(markdown, "text/plain"))
		}
		Some(RouteNode::Tool(tool_node)) => Pass(
			world
				.entity(tool_node.entity)
				.call::<Request, Response>(cx.input)
				.await?,
		),
		None => Fail(cx.input),
	}
	.xok()
}

/// Creates a markdown rendering tool for an entity's text content tree.
///
/// This tool traverses the entity and its descendants within the card
/// boundary, converting semantic markers to their markdown equivalents:
///
/// - [`Title`] → `# text` (heading level based on nesting)
/// - [`Paragraph`] → `text\n\n` (paragraph with trailing newlines)
/// - [`Important`] → `**text**` (bold)
/// - [`Emphasize`] → `*text*` (italic)
/// - [`Code`] → `` `text` `` (inline code)
/// - [`Quote`] → `"text"` (quoted)
/// - [`Link`] → `[text](url)`
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
/// 			.spawn((render_markdown(), content!["hello world"]))
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

/// System that renders an entity tree to markdown using CardQuery.
/// Used by the [`render_markdown`] tool, which renders relative to
/// its own entity (via card root resolution).
fn render_markdown_system(
	In(cx): In<ToolContext>,
	card_query: CardQuery,
	text_query: Query<&TextContent>,
	title_query: Query<(), With<Title>>,
	paragraph_query: Query<(), With<Paragraph>>,
	important_query: Query<(), With<Important>>,
	emphasize_query: Query<(), With<Emphasize>>,
	code_query: Query<(), With<Code>>,
	quote_query: Query<(), With<Quote>>,
	link_query: Query<&Link>,
	ancestors: Query<&ChildOf>,
) -> Result<String> {
	render_markdown_inner(
		cx.tool,
		true,
		&card_query,
		&text_query,
		&title_query,
		&paragraph_query,
		&important_query,
		&emphasize_query,
		&code_query,
		&quote_query,
		&link_query,
		&ancestors,
	)
	.xok()
}

/// System that renders a specific entity to markdown, starting from
/// that entity directly rather than resolving the card root first.
/// Used by [`render_markdown_for`].
fn render_markdown_for_entity(
	In(entity): In<Entity>,
	card_query: CardQuery,
	text_query: Query<&TextContent>,
	title_query: Query<(), With<Title>>,
	paragraph_query: Query<(), With<Paragraph>>,
	important_query: Query<(), With<Important>>,
	emphasize_query: Query<(), With<Emphasize>>,
	code_query: Query<(), With<Code>>,
	quote_query: Query<(), With<Quote>>,
	link_query: Query<&Link>,
	ancestors: Query<&ChildOf>,
) -> String {
	render_markdown_inner(
		entity,
		false,
		&card_query,
		&text_query,
		&title_query,
		&paragraph_query,
		&important_query,
		&emphasize_query,
		&code_query,
		&quote_query,
		&link_query,
		&ancestors,
	)
}

/// Core markdown rendering logic shared by the tool and the standalone
/// function.
///
/// When `resolve_card_root` is true, the iterator starts from the card
/// root of `entity` (tool behavior). When false, it starts directly
/// from `entity` (standalone behavior).
fn render_markdown_inner(
	entity: Entity,
	resolve_card_root: bool,
	card_query: &CardQuery,
	text_query: &Query<&TextContent>,
	title_query: &Query<(), With<Title>>,
	paragraph_query: &Query<(), With<Paragraph>>,
	important_query: &Query<(), With<Important>>,
	emphasize_query: &Query<(), With<Emphasize>>,
	code_query: &Query<(), With<Code>>,
	quote_query: &Query<(), With<Quote>>,
	link_query: &Query<&Link>,
	ancestors: &Query<&ChildOf>,
) -> String {
	let mut output = String::new();

	let iter: Box<dyn Iterator<Item = Entity>> = if resolve_card_root {
		Box::new(card_query.iter_dfs(entity))
	} else {
		Box::new(card_query.iter_dfs_from(entity))
	};

	for current in iter {
		if let Ok(text) = text_query.get(current) {
			let mut wrapped = text.as_str().to_string();

			// Apply inline wrappers from innermost to outermost
			if code_query.contains(current) {
				wrapped = format!("`{}`", wrapped);
			}
			if emphasize_query.contains(current) {
				wrapped = format!("*{}*", wrapped);
			}
			if important_query.contains(current) {
				wrapped = format!("**{}**", wrapped);
			}
			if quote_query.contains(current) {
				wrapped = format!("\"{}\"", wrapped);
			}
			if let Ok(link) = link_query.get(current) {
				let title = link
					.title
					.as_ref()
					.map(|title| format!(" \"{}\"", title))
					.unwrap_or_default();
				wrapped = format!("[{}]({}{})", wrapped, link.href, title);
			}

			// Structural elements
			if title_query.contains(current) {
				let title_level = ancestors
					.iter_ancestors_inclusive(current)
					.filter(|&ancestor| title_query.contains(ancestor))
					.count();
				let hashes = "#".repeat(title_level.min(6));
				wrapped = format!("{} {}\n\n", hashes, wrapped);
			} else if paragraph_query.contains(current) {
				wrapped = format!("{}\n\n", wrapped);
			}

			output.push_str(&wrapped);
		}
	}

	output
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn plain_text() {
		World::new()
			.spawn((render_markdown(), content!["hello world"]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("hello world");
	}

	#[test]
	fn multiple_segments() {
		World::new()
			.spawn((render_markdown(), content!["hello", " ", "world"]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("hello world");
	}

	#[test]
	fn important_text() {
		World::new()
			.spawn((render_markdown(), content![
				"hello ",
				(Important, "bold"),
				" text"
			]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("hello **bold** text");
	}

	#[test]
	fn emphasized_text() {
		World::new()
			.spawn((render_markdown(), content![
				"hello ",
				(Emphasize, "italic"),
				" text"
			]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("hello *italic* text");
	}

	#[test]
	fn code_text() {
		World::new()
			.spawn((render_markdown(), content![
				"use ",
				(Code, "println!"),
				" macro"
			]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("use `println!` macro");
	}

	#[test]
	fn quoted_text() {
		World::new()
			.spawn((render_markdown(), content!["he said ", (Quote, "hello")]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("he said \"hello\"");
	}

	#[test]
	fn link_without_title() {
		World::new()
			.spawn((render_markdown(), children![(
				TextContent::new("click here"),
				Link::new("https://example.com")
			)]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("[click here](https://example.com)");
	}

	#[test]
	fn link_with_title() {
		World::new()
			.spawn((render_markdown(), children![(
				TextContent::new("example"),
				Link::new("https://example.com").with_title("Example Site")
			)]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("[example](https://example.com \"Example Site\")");
	}


	#[test]
	fn combined_markers() {
		World::new()
			.spawn((render_markdown(), content![(
				Important,
				Emphasize,
				"bold italic"
			)]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("***bold italic***");
	}

	#[test]
	fn complex_composition() {
		World::new()
			.spawn((render_markdown(), content![
				"Welcome to ",
				(Important, "beet"),
				", the ",
				(Emphasize, "best"),
				" framework!"
			]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("Welcome to **beet**, the *best* framework!");
	}

	#[test]
	fn extension_trait() {
		World::new()
			.spawn((render_markdown(), content!["test"]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("test");
	}

	#[test]
	fn important_link() {
		World::new()
			.spawn((render_markdown(), children![(
				Important,
				TextContent::new("important link"),
				Link::new("https://example.com")
			)]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("[**important link**](https://example.com)");
	}

	#[test]
	fn all_markers_combined() {
		World::new()
			.spawn((render_markdown(), children![(
				Important,
				Emphasize,
				Code,
				Quote,
				TextContent::new("text"),
				Link::new("https://example.com")
			)]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("[\"***`text`***\"](https://example.com)");
	}

	#[test]
	fn title_renders_as_heading() {
		World::new()
			.spawn((render_markdown(), Title, TextContent::new("Hello World")))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("# Hello World\n\n");
	}

	#[test]
	fn nested_title_increments_level() {
		World::new()
			.spawn((
				render_markdown(),
				Title,
				TextContent::new("Outer"),
				children![(Title, TextContent::new("Inner"))],
			))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("# Outer\n\n## Inner\n\n");
	}

	#[test]
	fn paragraph_renders_with_newlines() {
		World::new()
			.spawn((
				render_markdown(),
				Paragraph,
				TextContent::new("A paragraph of text."),
			))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("A paragraph of text.\n\n");
	}

	#[test]
	fn mixed_structure() {
		World::new()
			.spawn((render_markdown(), children![
				(Title, TextContent::new("Welcome")),
				(Paragraph, TextContent::new("This is the intro."))
			]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("# Welcome\n\nThis is the intro.\n\n");
	}

	#[test]
	fn respects_card_boundary() {
		World::new()
			.spawn((render_markdown(), Card, children![
				(Paragraph, TextContent::new("Inside card")),
				// Nested card should not be rendered
				(Card, children![(
					Paragraph,
					TextContent::new("Inside nested card")
				)])
			]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("Inside card\n\n");
	}

	#[test]
	fn render_markdown_for_works() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, content!["hello ", (Important, "world")]))
			.id();

		let result = render_markdown_for(entity, &mut world);
		result.xpect_eq("hello **world**");
	}

	#[test]
	fn render_markdown_for_respects_card_boundary() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![
				(Paragraph, TextContent::new("visible")),
				(Card, children![(Paragraph, TextContent::new("hidden"))])
			]))
			.id();

		let result = render_markdown_for(entity, &mut world);
		result.xpect_eq("visible\n\n");
	}

	#[beet_core::test]
	async fn route_renders_card() {
		StackPlugin::world()
			.spawn((default_interface(), children![(
				card("about"),
				Paragraph,
				TextContent::new("About page"),
			)]))
			.call::<Request, Response>(Request::get("about"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.contains("About page")
			.xpect_true();
	}

	#[beet_core::test]
	async fn route_renders_root_card_on_empty_path() {
		StackPlugin::world()
			.spawn((default_interface(), children![(
				Card,
				Paragraph,
				TextContent::new("Root content"),
			)]))
			.call::<Request, Response>(Request::get(""))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("Root content");
	}

	#[beet_core::test]
	async fn route_renders_root_card_child() {
		let body = StackPlugin::world()
			.spawn((default_interface(), children![
				(Card, Title::with_text("My Server"), children![
					Paragraph::with_text("welcome!")
				]),
				card("about"),
			]))
			.call::<Request, Response>(Request::get(""))
			.await
			.unwrap()
			.unwrap_str()
			.await;
		body.contains("My Server").xpect_true();
		body.contains("welcome!").xpect_true();
	}

	#[beet_core::test]
	async fn route_calls_exchange_tool() {
		StackPlugin::world()
			.spawn((default_interface(), children![(
				PathPartial::new("add"),
				exchange_tool(|input: (i32, i32)| -> i32 { input.0 + input.1 }),
			)]))
			.call::<Request, Response>(
				Request::with_json("/add", &(10i32, 20i32)).unwrap(),
			)
			.await
			.unwrap()
			.body
			.into_json::<i32>()
			.await
			.unwrap()
			.xpect_eq(30);
	}
}
