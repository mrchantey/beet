//! Render semantic text content to markdown format.
//!
//! This module provides functionality to convert the semantic text representation
//! (using [`TextBlock`], [`TextContent`], and semantic markers) into markdown strings.
//!
//! # Example
//!
//! ```ignore
//! use beet_stack::prelude::*;
//! use beet_core::prelude::*;
//!
//! let mut world = DocumentPlugin::world();
//!
//! let entity = world.spawn(text![
//!     "Hello ",
//!     (Important, "world"),
//!     "!"
//! ]).id();
//!
//! let markdown = render_markdown(&world, entity).unwrap();
//! assert_eq!(markdown, "Hello **world**!");
//! ```

use crate::prelude::*;
use beet_core::prelude::*;


/// Renders an entity's text content tree to markdown.
///
/// This function traverses the entity and its children, converting
/// semantic markers to their markdown equivalents:
///
/// - [`Important`] → `**text**` (bold)
/// - [`Emphasize`] → `*text*` (italic)
/// - [`Code`] → `` `text` `` (inline code)
/// - [`Quote`] → `"text"` (quoted)
/// - [`Link`] → `[text](url)` or `[text](url "title")`
///
/// # Arguments
///
/// * `world` - The world containing the entities
/// * `entity` - The root entity to render (typically a [`TextBlock`])
///
/// # Returns
///
/// A markdown string representing the text content, or an error if
/// the entity structure is invalid.
pub fn render_markdown(world: &World, entity: Entity) -> Result<String> {
	let mut output = String::new();
	render_entity(world, entity, &mut output)?;
	Ok(output)
}

fn render_entity(world: &World, entity: Entity, output: &mut String) -> Result {
	let entity_ref = world.entity(entity);

	// Check if this entity has text content
	if let Some(text) = entity_ref.get::<TextContent>() {
		let content = text.as_str();

		// Apply semantic wrappers based on marker components
		let has_important = entity_ref.contains::<Important>();
		let has_emphasize = entity_ref.contains::<Emphasize>();
		let has_code = entity_ref.contains::<Code>();
		let has_quote = entity_ref.contains::<Quote>();
		let link = entity_ref.get::<Link>();

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
			wrapped = if let Some(title) = &link.title {
				format!("[{}]({} \"{}\")", wrapped, link.href, title)
			} else {
				format!("[{}]({})", wrapped, link.href)
			};
		}

		output.push_str(&wrapped);
	}

	// Process children
	if let Some(children) = entity_ref.get::<Children>() {
		for child in children.iter() {
			render_entity(world, child, output)?;
		}
	}

	Ok(())
}


/// Extension trait for rendering entities to markdown.
#[extend::ext(name = RenderMarkdownExt)]
pub impl World {
	/// Renders an entity's text content tree to markdown.
	///
	/// See [`render_markdown`] for details.
	fn render_markdown(&self, entity: Entity) -> Result<String> {
		render_markdown(self, entity)
	}
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn plain_text() {
		let mut world = World::new();
		let entity = world.spawn(text!["hello world"]).id();

		let result = render_markdown(&world, entity).unwrap();
		result.xpect_eq("hello world");
	}

	#[test]
	fn multiple_segments() {
		let mut world = World::new();
		let entity = world.spawn(text!["hello", " ", "world"]).id();

		let result = render_markdown(&world, entity).unwrap();
		result.xpect_eq("hello world");
	}

	#[test]
	fn important_text() {
		let mut world = World::new();
		let entity = world
			.spawn(text!["hello ", (Important, "bold"), " text"])
			.id();

		let result = render_markdown(&world, entity).unwrap();
		result.xpect_eq("hello **bold** text");
	}

	#[test]
	fn emphasized_text() {
		let mut world = World::new();
		let entity = world
			.spawn(text!["hello ", (Emphasize, "italic"), " text"])
			.id();

		let result = render_markdown(&world, entity).unwrap();
		result.xpect_eq("hello *italic* text");
	}

	#[test]
	fn code_text() {
		let mut world = World::new();
		let entity = world
			.spawn(text!["use ", (Code, "println!"), " macro"])
			.id();

		let result = render_markdown(&world, entity).unwrap();
		result.xpect_eq("use `println!` macro");
	}

	#[test]
	fn quoted_text() {
		let mut world = World::new();
		let entity = world.spawn(text!["he said ", (Quote, "hello")]).id();

		let result = render_markdown(&world, entity).unwrap();
		result.xpect_eq("he said \"hello\"");
	}

	#[test]
	fn link_without_title() {
		let mut world = World::new();
		let entity = world
			.spawn((TextBlock, children![(
				TextContent::new("click here"),
				Link::new("https://example.com")
			)]))
			.id();

		let result = render_markdown(&world, entity).unwrap();
		result.xpect_eq("[click here](https://example.com)");
	}

	#[test]
	fn link_with_title() {
		let mut world = World::new();
		let entity = world
			.spawn((TextBlock, children![(
				TextContent::new("example"),
				Link::new("https://example.com").with_title("Example Site")
			)]))
			.id();

		let result = render_markdown(&world, entity).unwrap();
		result.xpect_eq("[example](https://example.com \"Example Site\")");
	}

	#[test]
	fn combined_markers() {
		let mut world = World::new();
		let entity = world
			.spawn(text![(Important, Emphasize, "bold italic")])
			.id();

		let result = render_markdown(&world, entity).unwrap();
		// Important wraps Emphasize
		result.xpect_eq("***bold italic***");
	}

	#[test]
	fn complex_composition() {
		let mut world = World::new();
		let entity = world
			.spawn(text![
				"Welcome to ",
				(Important, "beet"),
				", the ",
				(Emphasize, "best"),
				" framework!"
			])
			.id();

		let result = render_markdown(&world, entity).unwrap();
		result.xpect_eq("Welcome to **beet**, the *best* framework!");
	}

	#[test]
	fn extension_trait() {
		let mut world = World::new();
		let entity = world.spawn(text!["test"]).id();

		let result = world.render_markdown(entity).unwrap();
		result.xpect_eq("test");
	}

	#[test]
	fn important_link() {
		let mut world = World::new();
		let entity = world
			.spawn((TextBlock, children![(
				Important,
				TextContent::new("important link"),
				Link::new("https://example.com")
			)]))
			.id();

		let result = render_markdown(&world, entity).unwrap();
		result.xpect_eq("[**important link**](https://example.com)");
	}

	#[test]
	fn all_markers_combined() {
		let mut world = World::new();
		let entity = world
			.spawn((TextBlock, children![(
				Important,
				Emphasize,
				Code,
				Quote,
				TextContent::new("text"),
				Link::new("https://example.com")
			)]))
			.id();

		let result = render_markdown(&world, entity).unwrap();
		// Order: code -> emphasize -> important -> quote -> link
		// Result: code wraps text, emphasize wraps code, important wraps emphasize
		result.xpect_eq("[\"***`text`***\"](https://example.com)");
	}
}
