//! Render semantic text content to markdown format.
//!
//! This module provides functionality to convert the semantic text representation
//! (using [`TextContent`] and semantic markers) into markdown strings.
//!
//! # Example
//!
//! ```ignore
//! use beet_stack::prelude::*;
//! use beet_core::prelude::*;
//!
//! let mut world = DocumentPlugin::world();
//!
//! let entity = world.spawn(content![
//!     "Hello ",
//!     (Important, "world"),
//!     "!"
//! ]).id();
//!
//! let markdown = render_markdown(&mut world, entity).unwrap();
//! assert_eq!(markdown, "Hello **world**!");
//! ```

use crate::prelude::*;
use beet_core::prelude::*;


/// Renders an entity's text content tree to markdown.
///
/// This function traverses the entity and its descendants within the card
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
/// # Arguments
///
/// * `world` - The world containing the entities
/// * `entity` - The root entity to render
///
/// # Returns
///
/// A markdown string representing the text content, or an error if
/// the entity structure is invalid.
pub fn render_markdown(world: &mut World, entity: Entity) -> Result<String> {
	world.run_system_cached_with(render_markdown_system, entity)?
}

/// System that renders an entity tree to markdown using CardQuery.
fn render_markdown_system(
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
) -> Result<String> {
	let mut output = String::new();

	// Use CardQuery DFS to traverse entities within the card boundary
	for current in card_query.iter_dfs_from(entity) {
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


/// Extension trait for rendering entities to markdown.
#[extend::ext(name = RenderMarkdownExt)]
pub impl World {
	/// Renders an entity's text content tree to markdown.
	///
	/// See [`render_markdown`] for details.
	fn render_markdown(&mut self, entity: Entity) -> Result<String> {
		render_markdown(self, entity)
	}
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn plain_text() {
		let mut world = World::new();
		let entity = world.spawn(content!["hello world"]).id();

		let result = render_markdown(&mut world, entity).unwrap();
		result.xpect_eq("hello world");
	}

	#[test]
	fn multiple_segments() {
		let mut world = World::new();
		let entity = world.spawn(content!["hello", " ", "world"]).id();

		let result = render_markdown(&mut world, entity).unwrap();
		result.xpect_eq("hello world");
	}

	#[test]
	fn important_text() {
		let mut world = World::new();
		let entity = world
			.spawn(content!["hello ", (Important, "bold"), " text"])
			.id();

		let result = render_markdown(&mut world, entity).unwrap();
		result.xpect_eq("hello **bold** text");
	}

	#[test]
	fn emphasized_text() {
		let mut world = World::new();
		let entity = world
			.spawn(content!["hello ", (Emphasize, "italic"), " text"])
			.id();

		let result = render_markdown(&mut world, entity).unwrap();
		result.xpect_eq("hello *italic* text");
	}

	#[test]
	fn code_text() {
		let mut world = World::new();
		let entity = world
			.spawn(content!["use ", (Code, "println!"), " macro"])
			.id();

		let result = render_markdown(&mut world, entity).unwrap();
		result.xpect_eq("use `println!` macro");
	}

	#[test]
	fn quoted_text() {
		let mut world = World::new();
		let entity = world.spawn(content!["he said ", (Quote, "hello")]).id();

		let result = render_markdown(&mut world, entity).unwrap();
		result.xpect_eq("he said \"hello\"");
	}

	#[test]
	fn link_without_title() {
		let mut world = World::new();
		let entity = world
			.spawn(children![(
				TextContent::new("click here"),
				Link::new("https://example.com")
			)])
			.id();

		let result = render_markdown(&mut world, entity).unwrap();
		result.xpect_eq("[click here](https://example.com)");
	}

	#[test]
	fn link_with_title() {
		let mut world = World::new();
		let entity = world
			.spawn(children![(
				TextContent::new("example"),
				Link::new("https://example.com").with_title("Example Site")
			)])
			.id();

		let result = render_markdown(&mut world, entity).unwrap();
		result.xpect_eq("[example](https://example.com \"Example Site\")");
	}


	#[test]
	fn combined_markers() {
		let mut world = World::new();
		let entity = world
			.spawn(content![(Important, Emphasize, "bold italic")])
			.id();

		let result = render_markdown(&mut world, entity).unwrap();
		// Important wraps Emphasize
		result.xpect_eq("***bold italic***");
	}

	#[test]
	fn complex_composition() {
		let mut world = World::new();
		let entity = world
			.spawn(content![
				"Welcome to ",
				(Important, "beet"),
				", the ",
				(Emphasize, "best"),
				" framework!"
			])
			.id();

		let result = render_markdown(&mut world, entity).unwrap();
		result.xpect_eq("Welcome to **beet**, the *best* framework!");
	}

	#[test]
	fn extension_trait() {
		let mut world = World::new();
		let entity = world.spawn(content!["test"]).id();

		let result = render_markdown(&mut world, entity).unwrap();
		result.xpect_eq("test");
	}

	#[test]
	fn important_link() {
		let mut world = World::new();
		let entity = world
			.spawn(children![(
				Important,
				TextContent::new("important link"),
				Link::new("https://example.com")
			)])
			.id();

		let result = render_markdown(&mut world, entity).unwrap();
		result.xpect_eq("[**important link**](https://example.com)");
	}

	#[test]
	fn all_markers_combined() {
		let mut world = World::new();
		let entity = world
			.spawn(children![(
				Important,
				Emphasize,
				Code,
				Quote,
				TextContent::new("text"),
				Link::new("https://example.com")
			)])
			.id();

		let result = render_markdown(&mut world, entity).unwrap();
		// Order: code -> emphasize -> important -> quote -> link
		// Result: code wraps text, emphasize wraps code, important wraps emphasize
		result.xpect_eq("[\"***`text`***\"](https://example.com)");
	}

	#[test]
	fn title_renders_as_heading() {
		let mut world = World::new();
		let entity = world.spawn((Title, TextContent::new("Hello World"))).id();

		let result = render_markdown(&mut world, entity).unwrap();
		result.xpect_eq("# Hello World\n\n");
	}

	#[test]
	fn nested_title_increments_level() {
		let mut world = World::new();
		let entity = world
			.spawn((Title, TextContent::new("Outer"), children![(
				Title,
				TextContent::new("Inner")
			)]))
			.id();

		let result = render_markdown(&mut world, entity).unwrap();
		result.xpect_eq("# Outer\n\n## Inner\n\n");
	}

	#[test]
	fn paragraph_renders_with_newlines() {
		let mut world = World::new();
		let entity = world
			.spawn((Paragraph, TextContent::new("A paragraph of text.")))
			.id();

		let result = render_markdown(&mut world, entity).unwrap();
		result.xpect_eq("A paragraph of text.\n\n");
	}

	#[test]
	fn mixed_structure() {
		let mut world = World::new();
		let root = world.spawn_empty().id();
		world.spawn((Title, TextContent::new("Welcome"), ChildOf(root)));
		world.spawn((
			Paragraph,
			TextContent::new("This is the intro."),
			ChildOf(root),
		));

		let result = render_markdown(&mut world, root).unwrap();
		result.xpect_eq("# Welcome\n\nThis is the intro.\n\n");
	}

	#[test]
	fn respects_card_boundary() {
		let mut world = World::new();
		let card = world.spawn(Card).id();
		world.spawn((
			Paragraph,
			TextContent::new("Inside card"),
			ChildOf(card),
		));

		// Nested card should not be rendered
		let nested_card = world.spawn((Card, ChildOf(card))).id();
		world.spawn((
			Paragraph,
			TextContent::new("Inside nested card"),
			ChildOf(nested_card),
		));

		let result = render_markdown(&mut world, card).unwrap();
		result.xpect_eq("Inside card\n\n");
	}
}
