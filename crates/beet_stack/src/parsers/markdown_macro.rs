//! The [`markdown!`] macro for spawning markdown as entities.
//!
//! Replaces [`content!`](crate::content) for test ergonomics by
//! parsing a markdown string and spawning the resulting entity tree
//! via [`MarkdownDiffer`](super::MarkdownDiffer).
//!
//! # Usage
//!
//! ```
//! use beet_stack::prelude::*;
//! use beet_core::prelude::*;
//!
//! let mut world = World::new();
//! let entity = markdown!(world, "# Hello **world**");
//!
//! // The entity now has Heading1 and Important children
//! let children = world.entity(entity).get::<Children>().unwrap();
//! children.len().xpect_eq(1);
//! ```
//!
//! # Differences from `content!`
//!
//! - Input is a markdown string rather than Rust expressions
//! - Always produces the container pattern (eg `Important` wraps
//!   `TextNode` children, never on the same entity)
//! - Cannot bind [`FieldRef`](crate::document::FieldRef) — use
//!   explicit bundle syntax for dynamic bindings

/// Spawn a markdown string as an entity tree.
///
/// Parses the markdown text with [`MarkdownDiffer`] and spawns the
/// resulting entities as children of a new root entity.
///
/// Returns the root [`Entity`].
///
/// # Examples
///
/// ```
/// use beet_stack::prelude::*;
/// use beet_core::prelude::*;
///
/// let mut world = World::new();
///
/// // Simple paragraph
/// let entity = markdown!(world, "Hello world");
///
/// // Heading with inline formatting
/// let entity = markdown!(world, "# Hello **world**");
///
/// // Multiple blocks
/// let entity = markdown!(world, "# Title\n\nA paragraph with *emphasis*.");
/// ```
#[macro_export]
macro_rules! markdown {
	($world:expr, $text:expr) => {{
		let root = $world.spawn_empty().id();
		$crate::prelude::MarkdownDiffer::new($text)
			.diff($world.entity_mut(root))
			.unwrap();
		root
	}};
}

/// Spawn a markdown string as an entity tree, returning the root entity.
///
/// Functional equivalent of the [`markdown!`] macro for contexts
/// where a function call is preferred.
#[cfg(feature = "markdown")]
pub fn spawn_markdown(
	world: &mut bevy::prelude::World,
	text: &str,
) -> bevy::prelude::Entity {
	use super::Parser;
	let root = world.spawn_empty().id();
	super::MarkdownDiffer::new(text)
		.diff(world.entity_mut(root))
		.unwrap();
	root
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn spawns_paragraph() {
		let mut world = World::new();
		let root = markdown!(world, "Hello world");

		let children = world.entity(root).get::<Children>().unwrap();
		children.len().xpect_eq(1);

		let para = *children.first().unwrap();
		world.entity(para).contains::<Paragraph>().xpect_true();
	}

	#[test]
	fn spawns_heading() {
		let mut world = World::new();
		let root = markdown!(world, "# Title");

		let children = world.entity(root).get::<Children>().unwrap();
		children.len().xpect_eq(1);

		let heading = *children.first().unwrap();
		world.entity(heading).contains::<Heading1>().xpect_true();

		let heading_children = world.entity(heading).get::<Children>().unwrap();
		let text_entity = *heading_children.first().unwrap();
		world
			.entity(text_entity)
			.get::<TextNode>()
			.unwrap()
			.as_str()
			.xpect_eq("Title");
	}

	#[test]
	fn spawns_bold_as_container() {
		let mut world = World::new();
		let root = markdown!(world, "**bold**");

		// Root -> Paragraph -> Important -> TextNode
		let children = world.entity(root).get::<Children>().unwrap();
		let para = *children.first().unwrap();
		let para_children = world.entity(para).get::<Children>().unwrap();
		let important = *para_children.first().unwrap();
		world.entity(important).contains::<Important>().xpect_true();
		// Important should NOT have TextNode on same entity
		world.entity(important).contains::<TextNode>().xpect_false();
		// TextNode should be a child of Important
		let important_children =
			world.entity(important).get::<Children>().unwrap();
		let text_entity = *important_children.first().unwrap();
		world
			.entity(text_entity)
			.get::<TextNode>()
			.unwrap()
			.as_str()
			.xpect_eq("bold");
	}

	#[test]
	fn spawns_italic_as_container() {
		let mut world = World::new();
		let root = markdown!(world, "*italic*");

		let children = world.entity(root).get::<Children>().unwrap();
		let para = *children.first().unwrap();
		let para_children = world.entity(para).get::<Children>().unwrap();
		let em = *para_children.first().unwrap();
		world.entity(em).contains::<Emphasize>().xpect_true();
		world.entity(em).contains::<TextNode>().xpect_false();
	}

	#[test]
	fn mixed_inline_formatting() {
		let mut world = World::new();
		let root = markdown!(world, "hello **bold** world");

		let children = world.entity(root).get::<Children>().unwrap();
		let para = *children.first().unwrap();
		let para_children = world.entity(para).get::<Children>().unwrap();
		// "hello " + Important("bold") + " world"
		para_children.len().xpect_eq(3);
	}

	#[test]
	fn multiple_blocks() {
		let mut world = World::new();
		let root = markdown!(world, "# Heading\n\nA paragraph.");

		let children = world.entity(root).get::<Children>().unwrap();
		children.len().xpect_eq(2);

		let heading = *children.first().unwrap();
		world.entity(heading).contains::<Heading1>().xpect_true();

		let para = children.iter().nth(1).unwrap();
		world.entity(para).contains::<Paragraph>().xpect_true();
	}

	#[test]
	fn spawns_link_as_container() {
		let mut world = World::new();
		let root = markdown!(world, "[click](https://example.com)");

		let children = world.entity(root).get::<Children>().unwrap();
		let para = *children.first().unwrap();
		let para_children = world.entity(para).get::<Children>().unwrap();
		let link_entity = *para_children.first().unwrap();
		let link = world.entity(link_entity).get::<Link>().unwrap();
		link.href.as_str().xpect_eq("https://example.com");
		// Link text should be a child, not on the same entity
		let link_children =
			world.entity(link_entity).get::<Children>().unwrap();
		let text_entity = *link_children.first().unwrap();
		world
			.entity(text_entity)
			.get::<TextNode>()
			.unwrap()
			.as_str()
			.xpect_eq("click");
	}

	#[test]
	fn spawn_markdown_function() {
		let mut world = World::new();
		let root = spawn_markdown(&mut world, "hello");

		let children = world.entity(root).get::<Children>().unwrap();
		children.len().xpect_eq(1);
	}

	#[test]
	fn code_block() {
		let mut world = World::new();
		let root = markdown!(world, "```rust\nfn main() {}\n```");

		let children = world.entity(root).get::<Children>().unwrap();
		let code = *children.first().unwrap();
		let cb = world.entity(code).get::<CodeBlock>().unwrap();
		cb.language.as_deref().xpect_eq(Some("rust"));
	}

	#[test]
	fn unordered_list() {
		let mut world = World::new();
		let root = markdown!(world, "- one\n- two\n- three");

		let children = world.entity(root).get::<Children>().unwrap();
		let list = *children.first().unwrap();
		let lm = world.entity(list).get::<ListMarker>().unwrap();
		lm.ordered.xpect_false();

		let list_children = world.entity(list).get::<Children>().unwrap();
		list_children.len().xpect_eq(3);
	}
}
