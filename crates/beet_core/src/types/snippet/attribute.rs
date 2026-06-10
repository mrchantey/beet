//! The [`Attribute`] markup node and the block-attribute helpers the
//! `rsx!` / `#[template]` lowerings reach for.
use crate::prelude::*;
#[cfg(feature = "tokens")]
use beet_core_macros::ToTokens;
use bevy::ecs::system::SystemParam;

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
pub struct Attribute(SmolStr);

impl Attribute {
	/// Construct an attribute with the given key.
	pub fn new(key: impl Into<SmolStr>) -> Self { Self(key.into()) }
	/// Whether the key names an event handler (`on*`).
	pub fn is_event(&self) -> bool { self.0.starts_with("on") }

	/// Build a single markup attribute as a [`Bundle`], for use as an `rsx!`
	/// block attribute: `<a {Attribute::bundle("href", url)}/>`. Attributes
	/// accumulate, so this sits alongside the element's literal attributes rather
	/// than replacing them.
	///
	/// It spawns the attribute as a *related* entity ([`AttributeOf`] the element)
	/// rather than `related!`-setting the whole [`Attributes`] target, so multiple
	/// block attributes and the element's literal attributes all coexist instead
	/// of the last one clobbering the rest.
	///
	/// Pair it with [`Attribute::bundle_option`] for an attribute that disappears
	/// when absent — the ergonomic form for optional props.
	pub fn bundle(
		key: impl Into<SmolStr>,
		value: impl Into<Value>,
	) -> impl Bundle {
		let key = key.into();
		let value = value.into();
		OnSpawn::new(move |entity| {
			let element = entity.id();
			entity.world_scope(move |world| {
				world.spawn((AttributeOf::new(element), Attribute::new(key), value));
			});
		})
	}

	/// A markup attribute that renders only when its value is [`Some`], for use as
	/// an `rsx!` block attribute:
	/// `<input {Attribute::bundle_option("name", name)}/>` where
	/// `name: Option<String>`.
	///
	/// A [`None`] renders nothing — unlike a defaulted empty string, which would
	/// emit an incorrect `name=""`. This is the ergonomic answer to "this prop is
	/// optional, so its attribute should be absent when unset".
	pub fn bundle_option(
		key: impl Into<SmolStr>,
		value: Option<impl Into<Value>>,
	) -> impl Bundle {
		OnSpawn::insert_option(value.map(|value| Attribute::bundle(key, value)))
	}
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
