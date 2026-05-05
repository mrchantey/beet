use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;

/// A token is like a typed pointer for the application layer.
/// Its key addresses the value in a store, and schema identifies the value type.
#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, SetWith, Get,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Token {
	/// Unique key for this token, ie `io.crates/beet_net/style/material/colors/Primary`
	key: TokenKey,
	/// Schema identifying the value type, ie `io.crates/bevy_color/color/Color`
	schema: TokenSchema,
}


impl Token {
	pub fn new(key: TokenKey, schema: TokenSchema) -> Self {
		Self { key, schema }
	}

	#[track_caller]
	pub fn new_inline(schema: TokenSchema) -> Self {
		Self {
			key: TokenKey::new_inline(),
			schema,
		}
	}
}


impl std::fmt::Display for Token {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.key.fmt(f)
	}
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, Get)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TypedValue {
	value: Value,
	/// Schema identifying the type, ie `io.crates/bevy_color/color/Color`
	schema: TokenSchema,
}

impl TypedValue {
	#[cfg(feature = "json")]
	pub fn new<T: Typed + Serialize>(value: T) -> Result<Self> {
		Self {
			value: Value::from_serde(&value)?,
			schema: TokenSchema::of::<T>(),
		}
		.xok()
	}
	#[cfg(feature = "json")]
	pub fn into_typed<T: Typed + DeserializeOwned>(&self) -> Result<T> {
		self.schema.assert_eq_ty::<T>()?;
		self.value.clone().into_serde::<T>()
	}
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TokenValue {
	Value(TypedValue),
	Token(Token),
}

impl TokenValue {
	pub fn schema(&self) -> &TokenSchema {
		match self {
			TokenValue::Value(value) => &value.schema,
			TokenValue::Token(token) => &token.schema,
		}
	}
}

impl From<TypedValue> for TokenValue {
	fn from(value: TypedValue) -> Self { Self::Value(value) }
}
impl<T> From<T> for TokenValue
where
	T: Into<Token>,
{
	fn from(token: T) -> Self { Self::Token(token.into()) }
}


/// A component holding the set of tokens applied to an element.
///
/// Used by non-CSS renderers (like charcell) to resolve styles.
///
/// ## Example
///
/// ```rust
/// # use beet_node::prelude::*;
/// # use beet_core::prelude::*;
/// // token!(MyToken, Color);
/// // let set = tokens![MyToken];
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deref, Reflect, Component)]
#[reflect(Component)]
pub struct TokenSet(HashSet<Token>);

impl TokenSet {
	pub fn new(items: impl IntoIterator<Item = Token>) -> Self {
		Self(items.into_iter().collect())
	}
}

/// Creates a [`TokenSet`], calling `.into()` for each item.
#[macro_export]
macro_rules! tokens {
	[$($child:expr),*$(,)?] => {
		$crate::prelude::TokenSet::new([$($child.into()),*])
	};
}


#[macro_export]
macro_rules! token {
	(
		$(#[$meta:meta])*
		$new_ty:ident,
		$schema_ty:ty
	) => {
		#[derive(::bevy::reflect::TypePath)]
		$(#[$meta])*
		pub struct $new_ty;
		impl Into<$crate::prelude::Token> for $new_ty {
			fn into(self) -> $crate::prelude::Token {
				$crate::prelude::Token::new(
					$crate::prelude::TokenKey::of::<Self>(),
					$crate::prelude::TokenSchema::of::<$schema_ty>(),
				)
			}
		}
	};
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_name() {
		Foo.xinto::<Token>()
			.key()
			.to_string()
			.xpect_eq("io.crates/beet_node/token/token/tests/Foo");
	}

	token!(
		/// Some cool type
		#[derive(Debug, Clone)]
		Foo,
		Color
	);
	token!(
		#[allow(unused)]
		Bar,
		Color
	);

	#[test]
	fn token_set_roundtrip() {
		let set = tokens![Foo, Bar];
		set.len().xpect_eq(2);
	}
}
