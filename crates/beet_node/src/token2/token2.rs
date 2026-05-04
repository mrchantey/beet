use crate::prelude::*;
use beet_core::prelude::*;

#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, SetWith, Get,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Token2 {
	key: Token2Key,
	schema: Token2Key,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, Get)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Token2KeyValue {
	key: Token2Key,
	value: TypedValue,
}
impl Token2KeyValue {
	pub fn new(key: Token2Key, value: TypedValue) -> Self {
		Self { key, value }
	}
	pub fn value_mut(&mut self) -> &mut TypedValue { &mut self.value }
}


#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Token2Key {
	Inline(SmolStr),
	Url(SmolStr),
	// Uuid(uuid::Uuid),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Token2Schema {
	ModulePath(SmolStr),
	Url(SmolStr),
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
#[relationship(relationship_target = Tokens)]
pub struct TokenOf(Entity);

impl TokenOf {
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
#[relationship_target(relationship = TokenOf,linked_spawn)]
pub struct Tokens(Vec<Entity>);
