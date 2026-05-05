use beet_core::prelude::*;
use bevy::reflect::Typed;

/// Trait for types that can be constructed from their token representation.
///
/// This is used by the CSS builder to resolve complex style values that reference
/// other tokens (like typography referencing typeface and weight tokens).
///
/// The `M` type parameter is a marker used to distinguish different impls.
#[cfg(feature = "serde")]
pub trait FromTokens<M>: Sized {
	type Tokens: Typed + serde::de::DeserializeOwned;
	fn from_value(
		value: &Value,
		entity: Entity,
		query: &super::StyleQuery,
	) -> Result<Self>
	where
		Self::Tokens: Sized,
	{
		let tokens = value.clone().into_serde::<Self::Tokens>()?;
		Self::from_tokens(tokens, entity, query)
	}
	fn from_tokens(
		tokens: Self::Tokens,
		entity: Entity,
		style_query: &super::StyleQuery,
	) -> Result<Self>;
}

/// Marker type for the blanket impl of `FromTokens` for types that don't
/// reference other tokens.
#[cfg(feature = "serde")]
pub struct SelfFromTokensMarker;

/// Blanket impl for types that represent themselves (no token references).
/// This allows simple types like `Color`, `f32`, `Length` to be used directly
/// without needing a separate Tokens struct.
#[cfg(feature = "serde")]
impl<T> FromTokens<SelfFromTokensMarker> for T
where
	T: Sized + Typed + serde::de::DeserializeOwned,
{
	type Tokens = Self;
	fn from_tokens(
		this: Self::Tokens,
		_entity: Entity,
		_style_query: &super::StyleQuery,
	) -> Result<Self> {
		this.xok()
	}
}
