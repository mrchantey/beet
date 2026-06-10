//! The basic markup-node data types shared by every front-end (the `rsx!`
//! macro, the BSX parser, serde) and read by the renderers.
//!
//! An [`Element`] is a single XML node (`div`, `span`, …); its
//! [`Attribute`]s are related entities ([`AttributeOf`]). [`Comment`] and
//! [`Doctype`] are the sibling node kinds. These are pure data: rendering them
//! to HTML or charcell lives in `beet_ui`.
use crate::prelude::*;
#[cfg(feature = "tokens")]
use beet_core_macros::ToTokens;
use bevy::ecs::system::SystemParam;

/// A single markup element node, ie `<div>`/`<span>`/`<p>`. Its tag name is the
/// inner string; its attributes are related [`Attribute`] entities and its
/// children are the usual [`Children`].
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Reflect,
	Component,
)]
#[reflect(Default, Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[component(immutable)]
// `Default` (empty tag) exists so `Element` picks up Bevy's blanket
// `Template`/`FromTemplate` impls (`Clone + Default`), making it usable as a
// scene template via `template_value`. The default is always overwritten.
pub struct Element(String);

impl Element {
	/// Construct an element with the given tag name.
	pub fn new(name: impl Into<String>) -> Self { Self(name.into()) }
	/// The tag name of this element, ie `div`, `span`, `p`.
	pub fn tag(&self) -> &str { &self.0 }
	/// Bundle this element with inner text, using [`OnSpawn`] to avoid
	/// clobbering other children.
	pub fn with_inner_text(self, text: &str) -> impl Bundle {
		(self, OnSpawn::insert_child(Value::Str(text.into())))
	}
}


/// A comment node. The inner string is the comment content excluding the
/// `<!--` and `-->` delimiters.
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	Reflect,
	Component,
)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[component(immutable)]
pub struct Comment(pub String);

impl Comment {
	/// Construct a comment from its (delimiter-free) content.
	pub fn new(content: impl Into<String>) -> Self { Self(content.into()) }
}

/// A doctype declaration. The inner string is the doctype value, usually
/// `"html"` for `<!DOCTYPE html>`.
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	Reflect,
	Component,
)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[component(immutable)]
pub struct Doctype(pub String);

impl Doctype {
	/// Construct a doctype from its value (ie `"html"`).
	pub fn new(value: impl Into<String>) -> Self { Self(value.into()) }
}

/// A single attribute on an [`Element`], stored as its own entity related to the
/// element via [`AttributeOf`]. The attribute's value lives in the required
/// [`Value`] component.
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	DerefMut,
	Reflect,
	Component,
)]
#[reflect(Default, Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[component(immutable)]
#[require(Value)]
// `Default` (empty key) exists for the same reason as [`Element`]: blanket
// `Template`/`FromTemplate`. The default is always overwritten.
pub struct Attribute(String);

impl Attribute {
	/// Construct an attribute with the given key.
	pub fn new(key: impl Into<String>) -> Self { Self(key.into()) }
	/// Whether the key names an event handler (`on*`).
	pub fn is_event(&self) -> bool { self.0.starts_with("on") }
}



/// The relationship pointing from an [`Attribute`] entity to the [`Element`] it
/// belongs to. The reverse [`Attributes`] collects an element's attributes.
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	Reflect,
	Component,
)]
#[reflect(Component)]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[relationship(relationship_target = Attributes)]
pub struct AttributeOf(Entity);

impl AttributeOf {
	/// Relate an attribute entity to its owning element.
	pub fn new(value: Entity) -> Self { Self(value) }
}

/// The set of [`Attribute`] entities belonging to an [`Element`] (the
/// relationship target of [`AttributeOf`]).
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	Reflect,
	Component,
)]
#[reflect(Component)]
#[relationship_target(relationship = AttributeOf,linked_spawn)]
pub struct Attributes(Vec<Entity>);



/// A [`SystemParam`] for reading an element's attributes by entity.
#[derive(SystemParam)]
pub struct AttributeQuery<'w, 's> {
	nodes: Query<'w, 's, (Entity, &'static Attributes)>,
	attributes: Query<'w, 's, (Entity, &'static Attribute, &'static Value)>,
}

impl AttributeQuery<'_, '_> {
	/// All `(entity, attribute, value)` tuples on the given element node.
	pub fn all(&self, node: Entity) -> Vec<(Entity, &Attribute, &Value)> {
		self.nodes.get(node).ok().map_or(vec![], |(_, attrs)| {
			attrs
				.iter()
				.filter_map(|attr| self.attributes.get(attr).ok())
				.collect()
		})
	}

	/// The `(entity, value)` of the attribute with `key`, if present.
	pub fn find(&self, node: Entity, key: &str) -> Option<(Entity, &Value)> {
		self.all(node)
			.into_iter()
			.find_map(|(entity, attribute, value)| {
				if **attribute == key {
					Some((entity, value))
				} else {
					None
				}
			})
	}

	/// The element's event attributes (keys starting with `on`).
	pub fn events(&self, node: Entity) -> Vec<(Entity, &Attribute, &Value)> {
		self.all(node)
			.into_iter()
			.filter(|(_, key, _)| key.is_event())
			.collect()
	}
}
