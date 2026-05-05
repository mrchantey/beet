use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;

/// Like a [`Document`] where branch nodes
/// are nested maps and leaf nodes are typed values.
/// It is perhaps more akin to a filesystem where files are
/// typed, than a freeform json value.
#[derive(Debug, Default, Clone, Deref, Reflect, Resource, Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TokenStore {
	tokens: HashMap<TokenKey, TokenValue>,
}


impl TokenStore {
	pub fn new() -> Self {
		Self {
			tokens: HashMap::new(),
		}
	}
	pub fn insert(
		&mut self,
		key: impl Into<Token>,
		value: impl Into<TokenValue>,
	) -> Result<&mut Self> {
		let value = value.into();
		let key = key.into();
		key.schema().assert_eq(value.schema())?;
		self.tokens.insert(key.key().clone(), value.into());
		self.xok()
	}
	fn with(
		mut self,
		key: impl Into<Token>,
		value: impl Into<TokenValue>,
	) -> Result<Self> {
		self.insert(key, value)?;
		self.xok()
	}
	pub fn with_token(
		self,
		key: impl Into<Token>,
		value: impl Into<Token>,
	) -> Result<Self> {
		self.with(key, value)
	}
	pub fn with_value(
		self,
		key: impl Into<Token>,
		value: impl Typed + Serialize,
	) -> Result<Self> {
		self.with(key, TypedValue::new(value)?)
	}
	#[track_caller]
	pub fn with_inline_value<T>(self, value: T) -> Result<Self>
	where
		T: Typed + Serialize,
	{
		let key = Token::new_inline(TokenSchema::of::<T>());
		self.with(key, TypedValue::new(value)?)
	}
	pub fn extend(
		mut self,
		rules: impl IntoIterator<Item = (TokenKey, TokenValue)>,
	) -> Self {
		self.tokens.extend(rules);
		self
	}

	pub fn get(&self, key: &Token) -> Result<&TokenValue> {
		match self.tokens.get(key.key()) {
			Some(value) => {
				key.schema().assert_eq(value.schema())?;
				Ok(value)
			}
			None => bevybail!("Token Not Found: `{key}`"),
		}
	}
	#[cfg(feature = "serde")]
	pub fn get_typed<T: Typed + serde::de::DeserializeOwned>(
		&self,
		key: &Token,
	) -> Result<T> {
		key.schema().assert_eq_ty::<T>()?;
		match self.get(key)? {
			TokenValue::Value(value) => value.into_typed::<T>(),
			TokenValue::Token(_) => {
				bevybail!("Expected Value, found Token: `{key}`")
			}
		}
	}
}


impl IntoIterator for TokenStore {
	type Item = (TokenKey, TokenValue);
	type IntoIter =
		bevy::platform::collections::hash_map::IntoIter<TokenKey, TokenValue>;
	fn into_iter(self) -> Self::IntoIter { self.tokens.into_iter() }
}

#[cfg(test)]
mod tests {
	use super::*;


	token!(CoolInts, Vec<u32>);
	token!(CoolFloats, Vec<f32>);
	token!(CoolStore, TokenStore);

	#[test]
	fn mismatch() {
		TokenStore::default()
			.with_value(CoolInts, vec![0., 1., 2.])
			.unwrap_err();
		TokenStore::default()
			.with_token(CoolInts, CoolFloats)
			.unwrap_err();
	}

	#[test]
	fn works() {
		let store = TokenStore::default()
			.with_value(CoolInts, vec![0u32, 1, 2])
			.unwrap();
		store.get(&CoolFloats.into()).xpect_err();
		store.get(&CoolInts.into()).xpect_ok();
		store.get_typed::<Vec<f32>>(&CoolInts.into()).xpect_err();
		store
			.get_typed::<Vec<u32>>(&CoolInts.into())
			.unwrap()
			.xpect_eq(vec![0u32, 1, 2]);
	}
	#[test]
	fn nested() {
		let store = TokenStore::default()
			.with_value(
				CoolStore,
				TokenStore::default()
					.with_value(CoolInts, vec![0u32, 1, 2])
					.unwrap(),
			)
			.unwrap();
		store
			.get_typed::<TokenStore>(&CoolStore.into())
			.unwrap()
			.get_typed::<Vec<u32>>(&CoolInts.into())
			.unwrap()
			.xpect_eq(vec![0u32, 1, 2]);
	}
}
