use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;

/// A token is like a typed pointer for the application layer.
/// Its path will store the value of its type in a document.
#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, SetWith, Get,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Token {
	/// Path to the value for this token
	/// ie `io.crates/beet_net/style/material/colors/PrimaryColor`
	key: TokenKey,
	/// Path to the token representing the value of this token,
	/// ie `io.crates/bevy_math/Color`
	schema: TokenKey,
	/// The path to the document
	document: DocumentPath,
}


impl Token {
	pub const fn new(
		key: TokenKey,
		schema: TokenKey,
		document: DocumentPath,
	) -> Self {
		Self {
			key,
			schema,
			document,
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
	/// Path to the token representing the type of this token,
	/// ie `io.crates/bevy_math/Color`
	schema: TokenKey,
}

impl TypedValue {
	pub fn new<T: Typed>(value: T) -> Result<Self> {
		Self {
			value: Value::from_reflect(&value)?,
			schema: TokenKey::of::<T>(),
		}
		.xok()
	}
	pub fn into_typed<T: Typed + FromReflect>(&self) -> Result<T> {
		self.schema.assert_eq_ty::<T>()?;
		self.value.into_reflect::<T>()
	}
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TokenValue {
	Value(TypedValue),
	Token(Token),
}

impl TokenValue {
	pub fn schema(&self) -> &TokenKey {
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



#[macro_export]
macro_rules! token {
	(
		$(#[$meta:meta])*
		$new_ty:ident,
		$schema_ty:ty
	) => {
		token!(
			$(#[$meta])* $new_ty,
			$schema_ty,
			$crate::prelude::DocumentPath::default()
		);
	};
	(
		$(#[$meta:meta])*
		$new_ty:ident,
		$schema_ty:ty,
		$doc_path: expr
	) => {
		#[derive(::bevy::reflect::TypePath)]
		$(#[$meta])*
		pub struct $new_ty;
		impl Into<$crate::prelude::Token> for $new_ty {
			fn into(self) -> $crate::prelude::Token {
				$crate::prelude::Token::new(
					$crate::prelude::TokenKey::of::<Self>(),
					$crate::prelude::TokenKey::of::<$schema_ty>(),
					$doc_path
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
			Color,
			DocumentPath::Ancestor
	);
	token!(
		#[allow(unused)]
		Bar,
		Color
	);
}
