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


#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Reflect,
	Get,
	GetMut,
	Deref,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TypedValue {
	#[deref]
	value: Value,
	/// Schema identifying the type, ie `bevy_color::color::Color`.
	#[get_mut(skip)]
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
	/// Construct directly from a [`Value`] and its [`TokenSchema`]. No
	/// type-check is performed, the caller is responsible for matching the
	/// value to the schema.
	pub fn from_value(value: Value, schema: TokenSchema) -> Self {
		Self { value, schema }
	}
	#[cfg(feature = "json")]
	pub fn into_typed<T: Typed + DeserializeOwned>(&self) -> Result<T> {
		self.schema.assert_eq_ty::<T>()?;
		self.value.clone().into_serde::<T>()
	}
	pub fn take(self) -> Value { self.value }
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
