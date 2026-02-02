//! Attribute types for DOM elements.
//!
//! This module provides components for representing HTML attributes,
//! including the attribute-element relationship and query helpers.

use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::SystemParam;


/// An attribute belonging to the target entity, which may be
/// an element or a node.
#[derive(Debug, Clone, Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = Attributes)]
pub struct AttributeOf(pub Entity);

impl AttributeOf {
	/// Creates a new attribute relationship pointing to the given entity.
	pub fn new(value: Entity) -> Self { Self(value) }
}

/// All attributes belonging to this entity, which may be
/// an element or a template.
#[derive(Debug, Clone, Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = AttributeOf,linked_spawn)]
pub struct Attributes(Vec<Entity>);


impl Attributes {
	/// returns the entity and value of the first attribute with the given key
	/// Returns the entity and value of the first attribute with the given key.
	pub fn find<'a>(
		&self,
		attrs: &'a Query<(Entity, &AttributeKey, Option<&TextNode>)>,
		key: &str,
	) -> Option<(Entity, Option<&'a TextNode>)> {
		self.iter().find_map(|entity| {
			let (attr_entity, item_key, value) = attrs.get(entity).ok()?;
			if item_key.as_str() == key {
				Some((attr_entity, value))
			} else {
				None
			}
		})
	}
}

/// A helper [`SystemParam`] for getting attributes from an element
#[derive(SystemParam)]
pub struct FindAttribute<'w, 's> {
	elements: Query<'w, 's, (Entity, &'static Attributes)>,
	attributes: Query<
		'w,
		's,
		(Entity, &'static AttributeKey, Option<&'static TextNode>),
	>,
}

impl FindAttribute<'_, '_> {
	/// Returns all attributes for the given entity.
	pub fn all<'a>(
		&'a self,
		entity: Entity,
	) -> Vec<(Entity, &'a AttributeKey, Option<&'a TextNode>)> {
		self.elements.get(entity).ok().map_or(vec![], |(_, attrs)| {
			attrs
				.iter()
				.filter_map(|attr| self.attributes.get(attr).ok())
				.collect()
		})
	}

	/// Returns all event attributes for the given entity.
	pub fn events<'a>(
		&'a self,
		entity: Entity,
	) -> Vec<(Entity, &'a AttributeKey)> {
		self.elements.get(entity).ok().map_or(vec![], |(_, attrs)| {
			attrs
				.iter()
				.filter_map(|attr| self.attributes.get(attr).ok())
				.filter(|(_, key, _)| key.is_event())
				.map(|(entity, key, _)| (entity, key))
				.collect()
		})
	}

	/// Finds an attribute by key on the given entity.
	pub fn find(
		&self,
		entity: Entity,
		key: &str,
	) -> Option<(Entity, Option<&TextNode>)> {
		self.elements
			.get(entity)
			.ok()
			.and_then(|(_, attrs)| attrs.find(&self.attributes, key))
	}
	/// Finds an attribute value by key on the given entity.
	pub fn find_value(
		&self,
		entity: Entity,
		key: &str,
	) -> Option<(Entity, &TextNode)> {
		self.find(entity, key)
			.and_then(|(attr_entity, value)| value.map(|v| (attr_entity, v)))
	}


	/// Collect all classes from the attributes of the given entity.
	pub fn classes(&self, entity: Entity) -> Vec<String> {
		self.find(entity, "class").map_or(vec![], |(_, value)| {
			value
				.map(|text| {
					text.as_str().split_whitespace().map(String::from).collect()
				})
				.unwrap_or_default()
		})
	}
}

/// An attribute key represented as a string.
///
/// ## Example
/// ```ignore
/// rsx!{<span "hidden"=true />};
/// ```
#[derive(
	Debug, Clone, PartialEq, Eq, Hash, Deref, DerefMut, Reflect, Component,
)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct AttributeKey(pub String);

impl AttributeKey {
	/// Creates a new attribute key with the given name.
	pub fn new(value: impl Into<String>) -> Self { Self(value.into()) }
	/// If key starts with 'on' its considered an event
	pub fn is_event(&self) -> bool { self.0.starts_with("on") }
}
