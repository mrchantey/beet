//! The [`markdown!`] macro for spawning markdown as entity bundles.
//!
//! Supports MDX-style `{}` interpolation for embedding bundle
//! expressions directly in markdown text. Brace groups are treated
//! as raw bundle expressions; everything else is collected as
//! markdown and parsed via [`MarkdownDiffer`] on spawn.
//!
//! # Usage
//!
//! ```
//! use beet_stack::prelude::*;
//! use beet_core::prelude::*;
//!
//! let mut world = World::new();
//!
//! // Single markdown string — parsed on spawn
//! let entity = world.spawn(markdown!("# Hello **world**")).id();
//! ```
//!
//! # MDX Interpolation
//!
//! ```ignore
//! // Raw token mode — `{}` groups become bundle expressions
//! markdown!{
//!     # Stock Counter
//!     Records in stock: { field_ref.clone().as_text() }
//!
//!     ## Tools
//!     {increment(field_ref)}
//! }
//!
//! // String literal mode — `{}` in the string are interpolated
//! markdown!(r#"
//!     # Stock Counter
//!     Records in stock: { field_ref.clone().as_text() }
//! "#)
//! ```

use crate::prelude::*;
use beet_core::prelude::*;

/// Convert a string into a bundle that parses markdown on spawn.
pub fn markdown(text: impl Into<String>) -> impl Bundle {
	let text = text.into();
	OnSpawn::new(move |entity: &mut EntityWorldMut| {
		let id = entity.id();
		entity.world_scope(|world| {
			MarkdownDiffer::new(&text)
				.diff(world.entity_mut(id))
				.unwrap();
		});
	})
}


/// Spawn markdown content as an entity bundle.
///
/// Supports three input forms:
///
/// 1. **String literal** — parsed as markdown on spawn:
///    ```ignore
///    markdown!("# Hello **world**")
///    ```
///
/// 2. **Raw tokens with `{}` interpolation** — brace groups become
///    bundle expressions, everything else becomes markdown:
///    ```ignore
///    markdown!{
///        # Title
///        {some_bundle_expr}
///        More *text*
///    }
///    ```
///
/// 3. **String literal with `{}` interpolation** — `{}` in the
///    string are parsed as bundle expressions:
///    ```ignore
///    markdown!(r#"
///        # Title
///        { some_bundle_expr }
///    "#)
///    ```
///
/// Escaped braces `{{like this}}` are treated as literal braces.
#[macro_export]
macro_rules! markdown {
	($($tt:tt)*) => {
		::beet_core::mdx!($crate; $($tt)*)
	};
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn spawns_paragraph() {
		let mut world = World::new();
		let root = world.spawn(markdown!("Hello world")).id();

		let children = world.entity(root).get::<Children>().unwrap();
		children.len().xpect_eq(1);

		let para = *children.first().unwrap();
		world.entity(para).contains::<Paragraph>().xpect_true();
	}

	#[test]
	fn spawns_heading() {
		let mut world = World::new();
		let root = world.spawn(markdown!("# Title")).id();

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
		let root = world.spawn(markdown!("**bold**")).id();

		// Root -> Paragraph -> Important -> TextNode
		let children = world.entity(root).get::<Children>().unwrap();
		let para = *children.first().unwrap();
		let para_children = world.entity(para).get::<Children>().unwrap();
		let important = *para_children.first().unwrap();
		world.entity(important).contains::<Important>().xpect_true();
		world.entity(important).contains::<TextNode>().xpect_false();
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
		let root = world.spawn(markdown!("*italic*")).id();

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
		let root = world.spawn(markdown!("hello **bold** world")).id();

		let children = world.entity(root).get::<Children>().unwrap();
		let para = *children.first().unwrap();
		let para_children = world.entity(para).get::<Children>().unwrap();
		// "hello " + Important("bold") + " world"
		para_children.len().xpect_eq(3);
	}

	#[test]
	fn multiple_blocks() {
		let mut world = World::new();
		let root = world.spawn(markdown!("# Heading\n\nA paragraph.")).id();

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
		let root = world.spawn(markdown!("[click](https://example.com)")).id();

		let children = world.entity(root).get::<Children>().unwrap();
		let para = *children.first().unwrap();
		let para_children = world.entity(para).get::<Children>().unwrap();
		let link_entity = *para_children.first().unwrap();
		let link = world.entity(link_entity).get::<Link>().unwrap();
		link.href.as_str().xpect_eq("https://example.com");
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
	fn code_block() {
		let mut world = World::new();
		let root = world.spawn(markdown!("```rust\nfn main() {}\n```")).id();

		let children = world.entity(root).get::<Children>().unwrap();
		let code = *children.first().unwrap();
		let cb = world.entity(code).get::<CodeBlock>().unwrap();
		cb.language.as_deref().xpect_eq(Some("rust"));
	}

	#[test]
	fn unordered_list() {
		let mut world = World::new();
		let root = world.spawn(markdown!("- one\n- two\n- three")).id();

		let children = world.entity(root).get::<Children>().unwrap();
		let list = *children.first().unwrap();
		let lm = world.entity(list).get::<ListMarker>().unwrap();
		lm.ordered.xpect_false();

		let list_children = world.entity(list).get::<Children>().unwrap();
		list_children.len().xpect_eq(3);
	}

	#[test]
	fn single_string_parses_markdown() {
		let mut world = World::new();
		let entity = world.spawn(markdown!("hello **world**")).id();

		// Single string: OnSpawn parses markdown onto the entity
		let children = world.entity(entity).get::<Children>().unwrap();
		let para = *children.first().unwrap();
		world.entity(para).contains::<Paragraph>().xpect_true();
		let para_children = world.entity(para).get::<Children>().unwrap();
		// "hello " + Important("world")
		para_children.len().xpect_eq(2);
	}
}
