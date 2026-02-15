use crate::prelude::*;
use beet_core::prelude::*;
use variadics_please::all_tuples;

/// Trait for types that can be converted into a bundle, including
/// primitives which are converted to a [`TextNode`].
trait IntoBundle<M> {
	/// Convert into a bundle suitable for spawning as a content segment.
	fn into_bundle(self) -> impl Bundle;
}

// String-like types get wrapped in TextNode
impl IntoBundle<Self> for &str {
	fn into_bundle(self) -> impl Bundle { TextNode::new(self) }
}

impl IntoBundle<Self> for String {
	fn into_bundle(self) -> impl Bundle { TextNode::new(self) }
}


// Specific impls for common bundle types to avoid overlapping with tuple impl
impl IntoBundle<Self> for TextNode {
	fn into_bundle(self) -> impl Bundle { self }
}

impl IntoBundle<Self> for Important {
	fn into_bundle(self) -> impl Bundle { self }
}

impl IntoBundle<Self> for Emphasize {
	fn into_bundle(self) -> impl Bundle { self }
}

impl IntoBundle<Self> for Code {
	fn into_bundle(self) -> impl Bundle { self }
}

impl IntoBundle<Self> for Quote {
	fn into_bundle(self) -> impl Bundle { self }
}

impl IntoBundle<Self> for FieldRef {
	fn into_bundle(self) -> impl Bundle { (self, TextNode::default()) }
}

pub struct TupleIntoBundleMarker;

macro_rules! impl_into_bundle_tuple {
 ($(#[$meta:meta])* $(($M:ident, $T:ident)),*) => {
  $(#[$meta])*
  impl<$($M, $T),*> IntoBundle<(TupleIntoBundleMarker, $($M,)*)> for ($($T,)*)
  where
   $($T: IntoBundle<$M>),*
  {
   #[allow(non_snake_case)]
   fn into_bundle(self) -> impl Bundle {
    let ($($T,)*) = self;
    ($($T.into_bundle(),)*)
   }
  }
 }
}


// Limited to 12 modifiers, matching Bevy's Bundle tuple impl limit
all_tuples!(
	// #[doc(fake_variadic)]
	impl_into_bundle_tuple,
	1,
	12,
	M,
	T
);


/// Convenience method for defining bundles with string types as well,
/// which are automatically converted to a [`TextNode`].
#[macro_export]
macro_rules! content {
	// Single expression - return directly without children! wrapper
	[$segment:expr $(,)?] => {
		$crate::prelude::into_bundle($segment)
	};
	// Multiple expressions - wrap in children!
	[$($segment:expr),+ $(,)?] => {
		::bevy::prelude::children![
			$($crate::prelude::into_bundle($segment)),+
		]
	};
}

#[inline]
#[allow(private_bounds)]
pub fn into_bundle<M>(value: impl IntoBundle<M>) -> impl Bundle {
	value.into_bundle()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn single_string_no_children() {
		let mut world = World::new();
		let entity = world.spawn(content!["hello"]).id();

		// Single expression should not create children
		world.entity(entity).get::<Children>().xpect_none();
		world
			.entity(entity)
			.get::<TextNode>()
			.unwrap()
			.as_str()
			.xpect_eq("hello");
	}

	#[test]
	fn multiple_strings() {
		let mut world = World::new();
		let entity = world.spawn(content!["hello", " ", "world"]).id();

		let children = world.entity(entity).get::<Children>().unwrap();
		children.len().xpect_eq(3);

		let texts: Vec<_> = children
			.iter()
			.map(|child| {
				world.entity(child).get::<TextNode>().unwrap().0.clone()
			})
			.collect();

		texts.xpect_eq(vec!["hello", " ", "world"]);
	}

	#[test]
	fn with_important() {
		let mut world = World::new();
		let entity = world.spawn(content!["normal ", (Important, "bold")]).id();

		let children = world.entity(entity).get::<Children>().unwrap();
		children.len().xpect_eq(2);

		let first = *children.first().unwrap();
		world.entity(first).contains::<Important>().xpect_false();

		let second = children.iter().nth(1).unwrap();
		world.entity(second).contains::<Important>().xpect_true();
		world
			.entity(second)
			.get::<TextNode>()
			.unwrap()
			.as_str()
			.xpect_eq("bold");
	}

	#[test]
	fn single_with_emphasize() {
		let mut world = World::new();
		let entity = world.spawn(content![(Emphasize, "italic")]).id();

		// Single expression - no children wrapper
		world.entity(entity).get::<Children>().xpect_none();
		world.entity(entity).contains::<Emphasize>().xpect_true();
	}

	#[test]
	fn with_code() {
		let mut world = World::new();
		let entity = world.spawn(content![(Code, "println!")]).id();

		world.entity(entity).contains::<Code>().xpect_true();
	}

	#[test]
	fn with_quote() {
		let mut world = World::new();
		let entity = world.spawn(content![(Quote, "quoted text")]).id();

		world.entity(entity).contains::<Quote>().xpect_true();
	}

	#[test]
	fn with_field_ref() {
		let mut world = World::new();
		let entity = world
			.spawn(content![
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
			.spawn(content![(
				Important,
				FieldRef::new("value").init_with(Value::I64(0))
			)])
			.id();

		// Single expression - no children wrapper
		world.entity(entity).contains::<Important>().xpect_true();
		world.entity(entity).contains::<FieldRef>().xpect_true();
		world.entity(entity).contains::<TextNode>().xpect_true();
	}

	#[test]
	fn multiple_modifiers() {
		let mut world = World::new();
		let entity = world
			.spawn(content![(Important, Emphasize, "bold italic")])
			.id();

		// Single expression - no children wrapper
		world.entity(entity).contains::<Important>().xpect_true();
		world.entity(entity).contains::<Emphasize>().xpect_true();
		world
			.entity(entity)
			.get::<TextNode>()
			.unwrap()
			.as_str()
			.xpect_eq("bold italic");
	}

	#[test]
	fn three_modifiers() {
		let mut world = World::new();
		let entity = world
			.spawn(content![(Important, Emphasize, Code, "all three")])
			.id();

		// Single expression - no children wrapper
		world.entity(entity).contains::<Important>().xpect_true();
		world.entity(entity).contains::<Emphasize>().xpect_true();
		world.entity(entity).contains::<Code>().xpect_true();
		world
			.entity(entity)
			.get::<TextNode>()
			.unwrap()
			.as_str()
			.xpect_eq("all three");
	}

	#[test]
	fn four_modifiers() {
		let mut world = World::new();
		let entity = world
			.spawn(content![(Important, Emphasize, Code, Quote, "all four")])
			.id();

		world.entity(entity).contains::<Important>().xpect_true();
		world.entity(entity).contains::<Emphasize>().xpect_true();
		world.entity(entity).contains::<Code>().xpect_true();
		world.entity(entity).contains::<Quote>().xpect_true();
		world
			.entity(entity)
			.get::<TextNode>()
			.unwrap()
			.as_str()
			.xpect_eq("all four");
	}

	#[test]
	fn complex_composition() {
		let mut world = World::new();
		let entity = world
			.spawn(content![
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
	fn text_macro_with_markers() {
		let mut world = World::new();
		let entity = world
			.spawn(content![(Important, Emphasize, "very important")])
			.id();

		world.entity(entity).contains::<Important>().xpect_true();
		world.entity(entity).contains::<Emphasize>().xpect_true();
	}
}
