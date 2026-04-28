use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;

/// Like a [`Document`] where branch nodes
/// are nested maps and leaf nodes are typed values.
/// It is perhaps more akin to a filesystem where files are
/// typed, than a freeform json value.
#[derive(Default, Deref, DerefMut, Resource, Reflect, Component)]
pub struct TokenStore {
	tokens: HashMap<TokenKey, TokenValue>,
}
impl TokenStore {
	pub fn new() -> Self {
		Self {
			tokens: HashMap::new(),
		}
	}
	pub fn with(
		mut self,
		key: impl Into<TokenKey>,
		value: impl Into<TokenValue>,
	) -> Result<Self> {
		let value = value.into();
		let key = key.into();
		key.assert_eq(value.schema())?;
		self.tokens.insert(key.into(), value.into());
		todo!("token schema comparison");
		// self.xok()
	}
	pub fn with_token<K: TypedTokenKey, V: TypedToken>(self) -> Result<Self> {
		self.with(K::token_key(), V::token())
	}
	pub fn with_value<K: TypedTokenKey>(
		self,
		value: impl Typed,
	) -> Result<Self> {
		self.with(K::token_key(), TypedValue::new(value)?)
	}
	pub fn extend(
		mut self,
		rules: impl IntoIterator<Item = (TokenKey, TokenValue)>,
	) -> Self {
		self.tokens.extend(rules);
		self
	}
}


// #[cfg(test)]
// mod tests {
// 	use super::*;

// 	#[test]
// 	fn mismatch() {
// 		let mut world = World::new();
// 		world.insert_resource(
// 			TokenStore::default()
// 				.with_value::<CoolNumbers>(vec![0u64, 1, 2])
// 				.unwrap(),
// 		);
// 	}

// 	#[test]
// 	fn works() {
// 		let mut world = World::new();
// 		world.insert_resource(
// 			TokenStore::default()
// 				.with_value::<CoolNumbers>(vec![0u32, 1, 2])
// 				.unwrap(),
// 		);
// 	}

// 	token!(
// 		#[allow(unused)]
// 		CoolNumbers,
// 		Vec<u32>
// 	);
// }
