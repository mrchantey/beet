use crate::prelude::*;
use beet_core::prelude::*;


#[derive(Debug, Clone, PartialEq, Eq, Deref, Reflect, Component)]
#[reflect(Component)]
pub struct TokenSet(HashSet<Token2>);

impl TokenSet {
	pub fn new(items: impl IntoIterator<Item = Token2>) -> Self {
		Self(items.into_iter().collect())
	}
}


#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, SetWith, Get,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Token2 {
	key: Token2Key,
	schema: Token2Schema,
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
	/// The schema is a bevy [`TypePath`] to a concrete rust type, with `::` delimiters
	RustType(SmolStr),
	/// The schema is a url
	Url(SmolStr),
}
