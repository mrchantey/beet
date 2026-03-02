use crate::prelude::*;
use beet_core::prelude::*;


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
}


/// Marker type Denoting that this node and its children should be rendered inside of comment notation,
/// and excluded from user-facing interfaces.
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
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[component(immutable)]
pub struct Comment;

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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
