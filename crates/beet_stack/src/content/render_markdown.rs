//! Render semantic text content to markdown format.
//!
//! This module provides functionality to convert the semantic text representation
//! (using [`TextContent`] and semantic markers) into markdown strings.
//! ```

use crate::prelude::*;
use beet_core::prelude::*;


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

/// System that renders an entity tree to markdown using CardQuery.
fn render_markdown_system(
	// In(entity): In<Entity>,
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
	let mut output = String::new();

	// Use CardQuery DFS to traverse entities within the card boundary
	for current in card_query.iter_dfs(cx.tool) {
		// Calculate title nesting level by counting Title ancestors
		let title_level = if title_query.contains(current) {
			ancestors
				.iter_ancestors(current)
				.filter(|&ancestor| title_query.contains(ancestor))
				.count() + 1
		} else {
			0
		};

		// Check if this entity has text content
		if let Ok(text) = text_query.get(current) {
			let content = text.as_str();

			// Apply semantic wrappers based on marker components
			let has_important = important_query.contains(current);
			let has_emphasize = emphasize_query.contains(current);
			let has_code = code_query.contains(current);
			let has_quote = quote_query.contains(current);
			let link = link_query.get(current).ok();
			let is_title = title_query.contains(current);
			let is_paragraph = paragraph_query.contains(current);

			// Build the wrapped content
			let mut wrapped = content.to_string();

			// Apply wrappers from innermost to outermost
			if has_code {
				wrapped = format!("`{}`", wrapped);
			}

			if has_emphasize {
				wrapped = format!("*{}*", wrapped);
			}

			if has_important {
				wrapped = format!("**{}**", wrapped);
			}

			if has_quote {
				wrapped = format!("\"{}\"", wrapped);
			}

			if let Some(link) = link {
				let title = link
					.title
					.as_ref()
					.map(|t| format!(" \"{}\"", t))
					.unwrap_or_default();
				wrapped = format!("[{}]({}{})", wrapped, link.href, title);
			}

			// Handle structural elements
			if is_title {
				let hashes = "#".repeat(title_level.min(6));
				wrapped = format!("{} {}\n\n", hashes, wrapped);
			} else if is_paragraph {
				wrapped = format!("{}\n\n", wrapped);
			}

			output.push_str(&wrapped);
		}
	}

	output.xok()
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn plain_text() {
		World::new()
			.spawn((render_markdown(), content!["hello world"]))
			.send_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("hello world");
	}

	#[test]
	fn multiple_segments() {
		World::new()
			.spawn((render_markdown(), content!["hello", " ", "world"]))
			.send_blocking::<(), String>(())
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
			.send_blocking::<(), String>(())
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
			.send_blocking::<(), String>(())
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
			.send_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("use `println!` macro");
	}

	#[test]
	fn quoted_text() {
		World::new()
			.spawn((render_markdown(), content!["he said ", (Quote, "hello")]))
			.send_blocking::<(), String>(())
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
			.send_blocking::<(), String>(())
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
			.send_blocking::<(), String>(())
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
			.send_blocking::<(), String>(())
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
			.send_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("Welcome to **beet**, the *best* framework!");
	}

	#[test]
	fn extension_trait() {
		World::new()
			.spawn((render_markdown(), content!["test"]))
			.send_blocking::<(), String>(())
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
			.send_blocking::<(), String>(())
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
			.send_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("[\"***`text`***\"](https://example.com)");
	}

	#[test]
	fn title_renders_as_heading() {
		World::new()
			.spawn((render_markdown(), Title, TextContent::new("Hello World")))
			.send_blocking::<(), String>(())
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
			.send_blocking::<(), String>(())
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
			.send_blocking::<(), String>(())
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
			.send_blocking::<(), String>(())
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
			.send_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("Inside card\n\n");
	}
}
