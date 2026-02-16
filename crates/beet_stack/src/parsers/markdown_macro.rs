//! The [`markdown!`] macro for spawning markdown as entity bundles.
//!
//! Uses the [`IntoMarkdownBundle`] trait to convert string literals
//! into bundles that parse markdown on spawn via [`OnSpawn`]. Bundle
//! expressions pass through unchanged, enabling interspersed
//! markdown text and explicit bundles.
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
//!
//! // Multiple segments — each becomes a child
//! let entity = world.spawn(markdown!(
//!     "# My Site",
//!     (Paragraph::with_text("interspersed bundle")),
//!     "And some *more text after*",
//! )).id();
//! ```
//!

use crate::prelude::*;
use beet_core::prelude::*;
use variadics_please::all_tuples;

/// Trait for types that can be converted into a markdown-aware bundle.
///
/// String types are parsed as markdown via [`MarkdownDiffer`] on
/// spawn. Other types pass through as regular bundles.
trait IntoMarkdownBundle<M> {
	/// Convert into a bundle for spawning as a markdown segment.
	fn into_markdown_bundle(self) -> impl Bundle;
}

// String-like types get parsed as markdown via OnSpawn
impl IntoMarkdownBundle<Self> for &str {
	fn into_markdown_bundle(self) -> impl Bundle { markdown(self) }
}



impl IntoMarkdownBundle<Self> for String {
	fn into_markdown_bundle(self) -> impl Bundle { markdown(self) }
}

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

pub struct ComponentIntoMarkdownBundleMarker;
impl<C: Component> IntoMarkdownBundle<ComponentIntoMarkdownBundleMarker> for C {
	fn into_markdown_bundle(self) -> impl Bundle { self }
}

// impl IntoMarkdownBundle<Self> for FieldRef {
// 	fn into_markdown_bundle(self) -> impl Bundle { (self, TextNode::default()) }
// }

pub struct TupleIntoMarkdownBundleMarker;

macro_rules! impl_into_markdown_bundle_tuple {
 ($(#[$meta:meta])* $(($M:ident, $T:ident)),*) => {
  $(#[$meta])*
  impl<$($M, $T),*> IntoMarkdownBundle<(TupleIntoMarkdownBundleMarker, $($M,)*)> for ($($T,)*)
  where
   $($T: IntoMarkdownBundle<$M>),*
  {
   #[allow(non_snake_case)]
   fn into_markdown_bundle(self) -> impl Bundle {
    let ($($T,)*) = self;
    ($($T.into_markdown_bundle(),)*)
   }
  }
 }
}

all_tuples!(impl_into_markdown_bundle_tuple, 1, 12, M, T);


/// Spawn markdown content as an entity bundle.
///
/// String literals are parsed as markdown via [`MarkdownDiffer`].
/// Bundle expressions pass through unchanged. Multiple segments
/// become children of the spawned entity.
///
/// # Examples
///
/// ```
/// use beet_stack::prelude::*;
/// use beet_core::prelude::*;
///
/// let mut world = World::new();
///
/// // Single markdown string
/// let entity = world.spawn(markdown!("# Hello **world**")).id();
///
/// // Multiple segments as children
/// let entity = world.spawn(markdown!(
///     "# Title",
///     (Paragraph::with_text("a bundle")),
///     "*emphasis*",
/// )).id();
/// ```
#[macro_export]
macro_rules! markdown {
	// Single expression — return directly without children! wrapper
	[$segment:expr $(,)?] => {
		$crate::prelude::into_markdown_bundle($segment)
	};
	// Multiple expressions — wrap in children!
	[$($segment:expr),+ $(,)?] => {
		::bevy::prelude::children![
			$($crate::prelude::into_markdown_bundle($segment)),+
		]
	};
}

#[inline]
#[allow(private_bounds)]
pub fn into_markdown_bundle<M>(
	value: impl IntoMarkdownBundle<M>,
) -> impl Bundle {
	value.into_markdown_bundle()
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
	fn multiple_segments_as_children() {
		let mut world = World::new();
		let root = world
			.spawn(markdown!(
				"# Title",
				"interspersed plain text",
				"*emphasis*",
			))
			.id();

		let children = world.entity(root).get::<Children>().unwrap();
		// Three segments become three children
		children.len().xpect_eq(3);

		// First child: OnSpawn parsed "# Title" → has Heading1 subchild
		let first = *children.first().unwrap();
		let first_children = world.entity(first).get::<Children>().unwrap();
		let heading = *first_children.first().unwrap();
		world.entity(heading).contains::<Heading1>().xpect_true();

		// Second child: OnSpawn parsed plain text → has Paragraph subchild
		let second = children.iter().nth(1).unwrap();
		let second_children = world.entity(second).get::<Children>().unwrap();
		let para = *second_children.first().unwrap();
		world.entity(para).contains::<Paragraph>().xpect_true();

		// Third child: OnSpawn parsed "*emphasis*" → has Paragraph subchild
		let third = children.iter().nth(2).unwrap();
		let third_children = world.entity(third).get::<Children>().unwrap();
		let para = *third_children.first().unwrap();
		world.entity(para).contains::<Paragraph>().xpect_true();
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
