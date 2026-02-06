//! The [`text!`] macro for composing semantic text content.
//!
//! This macro allows ergonomic construction of text blocks with
//! mixed static content, semantic markers, and dynamic field references.

use crate::prelude::*;
use beet_core::prelude::*;

/// Trait for types that can be converted into a text segment bundle.
pub trait IntoTextSegment {
	/// The bundle type produced by this conversion.
	type Bundle: Bundle;
	/// Convert into a bundle suitable for spawning as a text segment.
	fn into_text_bundle(self) -> Self::Bundle;
}

impl IntoTextSegment for &str {
	type Bundle = (TextContent,);
	fn into_text_bundle(self) -> Self::Bundle { (TextContent::new(self),) }
}

impl IntoTextSegment for String {
	type Bundle = (TextContent,);
	fn into_text_bundle(self) -> Self::Bundle { (TextContent::new(self),) }
}

impl IntoTextSegment for FieldRef {
	type Bundle = (TextContent, FieldRef);
	fn into_text_bundle(self) -> Self::Bundle { (TextContent::default(), self) }
}

impl IntoTextSegment for (Important, &str) {
	type Bundle = (Important, TextContent);
	fn into_text_bundle(self) -> Self::Bundle {
		(Important, TextContent::new(self.1))
	}
}

impl IntoTextSegment for (Emphasize, &str) {
	type Bundle = (Emphasize, TextContent);
	fn into_text_bundle(self) -> Self::Bundle {
		(Emphasize, TextContent::new(self.1))
	}
}

impl IntoTextSegment for (Code, &str) {
	type Bundle = (Code, TextContent);
	fn into_text_bundle(self) -> Self::Bundle {
		(Code, TextContent::new(self.1))
	}
}

impl IntoTextSegment for (Quote, &str) {
	type Bundle = (Quote, TextContent);
	fn into_text_bundle(self) -> Self::Bundle {
		(Quote, TextContent::new(self.1))
	}
}

impl IntoTextSegment for (Important, Emphasize, &str) {
	type Bundle = (Important, Emphasize, TextContent);
	fn into_text_bundle(self) -> Self::Bundle {
		(Important, Emphasize, TextContent::new(self.2))
	}
}

impl IntoTextSegment for (Important, FieldRef) {
	type Bundle = (Important, TextContent, FieldRef);
	fn into_text_bundle(self) -> Self::Bundle {
		(Important, TextContent::default(), self.1)
	}
}

impl IntoTextSegment for (Emphasize, FieldRef) {
	type Bundle = (Emphasize, TextContent, FieldRef);
	fn into_text_bundle(self) -> Self::Bundle {
		(Emphasize, TextContent::default(), self.1)
	}
}

impl IntoTextSegment for (Code, FieldRef) {
	type Bundle = (Code, TextContent, FieldRef);
	fn into_text_bundle(self) -> Self::Bundle {
		(Code, TextContent::default(), self.1)
	}
}

impl IntoTextSegment for (Quote, FieldRef) {
	type Bundle = (Quote, TextContent, FieldRef);
	fn into_text_bundle(self) -> Self::Bundle {
		(Quote, TextContent::default(), self.1)
	}
}

impl IntoTextSegment for (Important, Emphasize, FieldRef) {
	type Bundle = (Important, Emphasize, TextContent, FieldRef);
	fn into_text_bundle(self) -> Self::Bundle {
		(Important, Emphasize, TextContent::default(), self.2)
	}
}

/// Compose text content with semantic markers and dynamic fields.
///
/// Creates a [`TextBlock`] entity with child [`TextContent`] entities,
/// each optionally enhanced with semantic components like [`Important`],
/// [`Emphasize`], or dynamic [`FieldRef`] bindings.
///
/// # Usage
///
/// ```ignore
/// use beet_stack::prelude::*;
/// use beet_core::prelude::*;
///
/// // Static text segments
/// let block = text!["Hello, ", "world!"];
///
/// // With semantic markers using tuples
/// let block = text![
///     "Welcome to ",
///     (Important, "beet"),
///     "!"
/// ];
///
/// // With dynamic field references
/// let block = text![
///     "Count: ",
///     FieldRef::new("count").init_with(Value::I64(0))
/// ];
///
/// // Combined usage
/// let block = text![
///     "The ",
///     (Emphasize, "current"),
///     " value is: ",
///     (Important, FieldRef::new("value").init_with(Value::I64(42)))
/// ];
/// ```
///
/// # Generated Structure
///
/// The macro produces a bundle containing:
/// - A [`TextBlock`] marker on the parent entity
/// - Child entities with [`TextContent`] and any specified semantic components
///
/// [`TextBlock`]: crate::content::TextBlock
/// [`TextContent`]: crate::content::TextContent
/// [`Important`]: crate::content::Important
/// [`Emphasize`]: crate::content::Emphasize
/// [`FieldRef`]: crate::document::FieldRef
#[macro_export]
macro_rules! text {
	[$($segment:expr),+ $(,)?] => {
		(
			$crate::prelude::TextBlock,
			::bevy::prelude::children![
				$($crate::prelude::IntoTextSegment::into_text_bundle($segment)),+
			]
		)
	};
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn single_string() {
		let mut world = World::new();
		let entity = world.spawn(text!["hello"]).id();

		world.entity(entity).contains::<TextBlock>().xpect_true();

		let children = world.entity(entity).get::<Children>().unwrap();
		children.len().xpect_eq(1);

		let &child = children.first().unwrap();
		world
			.entity(child)
			.get::<TextContent>()
			.unwrap()
			.as_str()
			.xpect_eq("hello");
	}

	#[test]
	fn multiple_strings() {
		let mut world = World::new();
		let entity = world.spawn(text!["hello", " ", "world"]).id();

		let children = world.entity(entity).get::<Children>().unwrap();
		children.len().xpect_eq(3);

		let texts: Vec<_> = children
			.iter()
			.map(|child| {
				world.entity(child).get::<TextContent>().unwrap().0.clone()
			})
			.collect();

		texts.xpect_eq(vec!["hello", " ", "world"]);
	}

	#[test]
	fn with_important_marker() {
		let mut world = World::new();
		let entity = world.spawn(text!["normal ", (Important, "bold")]).id();

		let children = world.entity(entity).get::<Children>().unwrap();
		children.len().xpect_eq(2);

		let &first = children.first().unwrap();
		world.entity(first).contains::<Important>().xpect_false();

		let second = children.iter().nth(1).unwrap();
		world.entity(second).contains::<Important>().xpect_true();
		world
			.entity(second)
			.get::<TextContent>()
			.unwrap()
			.as_str()
			.xpect_eq("bold");
	}

	#[test]
	fn with_emphasize_marker() {
		let mut world = World::new();
		let entity = world.spawn(text![(Emphasize, "italic")]).id();

		let children = world.entity(entity).get::<Children>().unwrap();
		let &child = children.first().unwrap();
		world.entity(child).contains::<Emphasize>().xpect_true();
	}

	#[test]
	fn with_field_ref() {
		let mut world = World::new();
		let entity = world
			.spawn(text![
				"Value: ",
				FieldRef::new("count").init_with(Value::I64(42))
			])
			.id();

		let children = world.entity(entity).get::<Children>().unwrap();
		children.len().xpect_eq(2);

		let second = children.iter().nth(1).unwrap();
		let field_ref = world.entity(second).get::<FieldRef>().unwrap();
		field_ref
			.field_path
			.xpect_eq(vec![FieldPath::ObjectKey("count".into())]);
	}

	#[test]
	fn with_important_field_ref() {
		let mut world = World::new();
		let entity = world
			.spawn(text![(
				Important,
				FieldRef::new("value").init_with(Value::I64(0))
			)])
			.id();

		let children = world.entity(entity).get::<Children>().unwrap();
		let &child = children.first().unwrap();

		world.entity(child).contains::<Important>().xpect_true();
		world.entity(child).contains::<FieldRef>().xpect_true();
		world.entity(child).contains::<TextContent>().xpect_true();
	}

	#[test]
	fn complex_composition() {
		let mut world = World::new();
		let entity = world
			.spawn(text![
				"Welcome ",
				(Important, "user"),
				", your score is: ",
				(Emphasize, FieldRef::new("score").init_with(Value::I64(100)))
			])
			.id();

		let children = world.entity(entity).get::<Children>().unwrap();
		children.len().xpect_eq(4);
	}

	#[test]
	fn multiple_markers() {
		let mut world = World::new();
		let entity = world
			.spawn(text![(Important, Emphasize, "very important")])
			.id();

		let children = world.entity(entity).get::<Children>().unwrap();
		let &child = children.first().unwrap();

		world.entity(child).contains::<Important>().xpect_true();
		world.entity(child).contains::<Emphasize>().xpect_true();
		world
			.entity(child)
			.get::<TextContent>()
			.unwrap()
			.as_str()
			.xpect_eq("very important");
	}
}
