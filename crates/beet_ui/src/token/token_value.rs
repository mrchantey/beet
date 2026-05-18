use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TokenValue {
	Value(TypedValue),
	Token(Token),
}

impl TokenValue {
	#[cfg(feature = "serde")]
	pub fn value<T: Typed + Serialize>(value: T) -> Result<Self> {
		Self::Value(TypedValue::new(value)?).xok()
	}
	pub fn token(token: impl Into<Token>) -> Self { Self::Token(token.into()) }
}

impl TokenValue {
	pub fn schema(&self) -> &TokenSchema {
		match self {
			TokenValue::Value(value) => value.schema(),
			TokenValue::Token(token) => token.schema(),
		}
	}
}

impl<T> From<T> for TokenValue
where
	T: Into<Token>,
{
	fn from(token: T) -> Self { Self::Token(token.into()) }
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
			// `Value::from_serde` round-trips through `serde_json`, which has no
			// signed/unsigned distinction, so a positive `i32` would land as
			// `Value::Uint`. We still know the Rust type here, so restore the
			// signed variant when the schema is a signed integer.
			value: coerce_signed::<T>(Value::from_serde(&value)?),
			schema: TokenSchema::of::<T>(),
		}
		.xok()
	}
	#[cfg(feature = "json")]
	pub fn into_typed<T: Typed + DeserializeOwned>(&self) -> Result<T> {
		self.schema.assert_eq_ty::<T>()?;
		self.value.clone().into_serde::<T>()
	}
	pub fn take(self) -> Value { self.value }
}


/// Restore the signed integer variant lost by the lossy `serde_json` hop in
/// [`TypedValue::new`]. Only scalar signed integers need this; nested numbers
/// are recovered on the way out via typed deserialization.
fn coerce_signed<T: bevy::reflect::TypePath>(value: Value) -> Value {
	match (T::type_path(), value) {
		("i8" | "i16" | "i32" | "i64" | "isize", Value::Uint(u)) => {
			Value::Int(u as i64)
		}
		(_, value) => value,
	}
}

impl From<TypedValue> for TokenValue {
	fn from(value: TypedValue) -> Self { Self::Value(value) }
}
// impl<T> From<T> for TypedValue
// where
// 	T: Typed + Serialize,
// {
// 	fn from(value: T) -> Self {
// 		Self::new(value).expect("failed to serialize value")
// 	}
// }


pub trait IntoTokenValue<M> {
	fn into_token_value(self) -> TokenValue;
}

// pub struct

pub struct IntoTokenValueMarker;

impl<T> IntoTokenValue<IntoTokenValueMarker> for T
where
	TokenValue: From<T>,
{
	fn into_token_value(self) -> TokenValue { self.into() }
}

impl<T: Typed + Serialize> IntoTokenValue<Self> for T {
	fn into_token_value(self) -> TokenValue {
		TokenValue::Value(
			TypedValue::new(self).expect("failed to serialize value"),
		)
	}
}
