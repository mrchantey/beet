use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::query::QueryEntityError;
use std::borrow::Cow;


#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, Component,
)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[component(immutable)]
pub struct Element(String);

impl Element {
	pub fn new(name: impl Into<String>) -> Self { Self(name.into()) }
	/// The tag name of this element, ie `div`, `span`, `p`.
	pub fn name(&self) -> &str { &self.0 }
}

#[derive(SystemParam)]
pub struct ElementQuery<'w, 's> {
	elements:
		Query<'w, 's, (Entity, &'static Element, Option<&'static Attributes>)>,
	attributes: Query<'w, 's, (Entity, &'static Attribute, &'static Value)>,
}

impl ElementQuery<'_, '_> {
	pub fn iter(&self) -> impl Iterator<Item = ElementView> {
		self.elements.iter().map(|(entity, element, attrs)| {
			let attributes = attrs
				.map(|attrs| {
					attrs
						.iter()
						.filter_map(|attr| self.attributes.get(attr).ok())
						.collect()
				})
				.unwrap_or_default();
			ElementView::new(entity, element, attributes)
		})
	}

	pub fn get(&self, entity: Entity) -> Result<ElementView, QueryEntityError> {
		self.elements.get(entity).map(|(entity, element, attrs)| {
			let attributes = attrs
				.map(|attrs| {
					attrs
						.iter()
						.filter_map(|attr| self.attributes.get(attr).ok())
						.collect()
				})
				.unwrap_or_default();
			ElementView::new(entity, element, attributes)
		})
	}

	pub fn get_as<'a, T>(&'a self, entity: Entity) -> Result<T>
	where
		T: TryFrom<&'a ElementView<'a>, Error = BevyError>,
	{
		let element = self.get(entity)?;
		element.try_as::<T>()
	}
}


/// Read-only view of an element and its attributes, provided to
/// [`NodeVisitor::visit_element`] for convenient attribute lookup.
pub struct ElementView<'a> {
	/// The entity of this element.
	pub entity: Entity,
	/// The element component.
	pub element: &'a Element,
	/// Attribute triples `(entity, key, value)` for this element.
	pub attributes: Vec<(Entity, &'a Attribute, &'a Value)>,
}

impl<'a> ElementView<'a> {
	/// Create a new view from an element reference and its attributes.
	pub fn new(
		entity: Entity,
		element: &'a Element,
		attributes: Vec<(Entity, &'a Attribute, &'a Value)>,
	) -> Self {
		Self {
			entity,
			element,
			attributes,
		}
	}

	pub fn try_as<'b, T: TryFrom<&'b Self>>(&'b self) -> Result<T, T::Error> {
		T::try_from(self)
	}

	/// The tag name of this element, ie `div`, `span`, `p`.
	pub fn name(&self) -> &str { self.element.name() }

	/// Look up the first attribute matching `key` and return its value.
	pub fn attribute(&self, key: &str) -> Option<&'a Value> {
		self.attributes
			.iter()
			.find(|(_, attr, _)| attr.as_str() == key)
			.map(|(_, _, val)| *val)
	}

	/// Look up the first attribute matching `key` and return its
	/// `(entity, value)` pair.
	pub fn attribute_with_entity(
		&self,
		key: &str,
	) -> Option<(Entity, &'a Value)> {
		self.attributes
			.iter()
			.find(|(_, attr, _)| attr.as_str() == key)
			.map(|(entity, _, val)| (*entity, *val))
	}

	/// Look up an attribute and convert its value to a [`String`].
	/// Returns an empty string when the attribute is absent.
	pub fn attribute_string(&self, key: &str) -> String {
		self.attribute(key)
			.map(|val| val.to_string())
			.unwrap_or_default()
	}

	/// Extract the `start` attribute as a `usize` for ordered lists.
	/// Defaults to `1` when absent or not numeric.
	pub fn ol_start(&self) -> usize {
		self.attribute("start")
			.and_then(|val| match val {
				Value::Uint(num) => Some(*num as usize),
				Value::Int(num) => Some(*num as usize),
				_ => None,
			})
			.unwrap_or(1)
	}
}


pub struct LinkView<'a> {
	element: &'a ElementView<'a>,
	href: &'a str,
}

impl<'a> TryFrom<&'a ElementView<'a>> for LinkView<'a> {
	type Error = BevyError;
	fn try_from(
		value: &'a ElementView<'a>,
	) -> std::result::Result<Self, Self::Error> {
		if value.name() == "a" {
			let href = value
				.attribute("href")
				.and_then(|val| val.try_string())
				.unwrap_or("");

			Ok(Self {
				element: value,
				href,
			})
		} else {
			bevybail!("not an anchor element")
		}
	}
}


/// An HTML comment node. The inner string is the comment content
/// excluding the `<!--` and `-->` delimiters.
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
	pub fn new(content: impl Into<String>) -> Self { Self(content.into()) }
}

/// An HTML doctype declaration. The inner string is the doctype value,
/// usually `"html"` for `<!DOCTYPE html>`.
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
	pub fn new(value: impl Into<String>) -> Self { Self(value.into()) }
}

#[derive(
	Debug,
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
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[component(immutable)]
#[require(Value)]
pub struct Attribute(String);

impl Attribute {
	pub fn new(key: impl Into<String>) -> Self { Self(key.into()) }
	pub fn is_event(&self) -> bool { self.0.starts_with("on") }
}



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
	pub fn new(value: Entity) -> Self { Self(value) }
}

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



#[derive(SystemParam)]
pub struct AttributeQuery<'w, 's> {
	nodes: Query<'w, 's, (Entity, &'static Attributes)>,
	attributes: Query<'w, 's, (Entity, &'static Attribute, &'static Value)>,
}

impl AttributeQuery<'_, '_> {
	pub fn all(&self, node: Entity) -> Vec<(Entity, &Attribute, &Value)> {
		self.nodes.get(node).ok().map_or(vec![], |(_, attrs)| {
			attrs
				.iter()
				.filter_map(|attr| self.attributes.get(attr).ok())
				.collect()
		})
	}

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

	pub fn events(&self, node: Entity) -> Vec<(Entity, &Attribute, &Value)> {
		self.all(node)
			.into_iter()
			.filter(|(_, key, _)| key.is_event())
			.collect()
	}
}


/// The default set of HTML block-level element names.
///
/// Shared by renderers that need to distinguish block vs inline elements
/// for whitespace and layout decisions.
pub fn default_block_elements() -> Vec<Cow<'static, str>> {
	vec![
		"address".into(),
		"article".into(),
		"aside".into(),
		"blockquote".into(),
		"details".into(),
		"dialog".into(),
		"dd".into(),
		"div".into(),
		"dl".into(),
		"dt".into(),
		"fieldset".into(),
		"figcaption".into(),
		"figure".into(),
		"footer".into(),
		"form".into(),
		"h1".into(),
		"h2".into(),
		"h3".into(),
		"h4".into(),
		"h5".into(),
		"h6".into(),
		"header".into(),
		"hgroup".into(),
		"hr".into(),
		"li".into(),
		"main".into(),
		"nav".into(),
		"ol".into(),
		"p".into(),
		"pre".into(),
		"search".into(),
		"section".into(),
		"table".into(),
		"ul".into(),
	]
}
