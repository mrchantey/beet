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
	/// The path to the document
	document: DocumentPath,
	/// Path to the token representing the value of this token,
	/// ie `io.crates/bevy_math/Color`
	schema: TokenKey,
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
}


impl Token {
	pub fn new(key: TokenKey, schema: TokenKey) -> Self {
		Self {
			key,
			schema,
			document: default(),
		}
	}

	/// Create new token, using `Token` for the field path
	pub fn of<Path: TypePath, Schema: TypePath>() -> Self {
		Self {
			key: TokenKey::of::<Path>(),
			schema: TokenKey::of::<Schema>(),
			document: default(),
		}
	}
}

/// A type which represents a token, see `token2!` for defining.
pub trait TypedToken {
	fn schema() -> TokenKey;
	fn key() -> TokenKey;
	fn token() -> Token {
		Token {
			key: Self::key(),
			schema: Self::schema(),
			document: default(),
		}
	}
}

impl<T: TypedToken> From<T> for Token {
	fn from(_: T) -> Self { T::token() }
}
impl<T: TypedToken> From<T> for TokenKey {
	fn from(_: T) -> Self { T::key() }
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
impl From<Token> for TokenValue {
	fn from(token: Token) -> Self { Self::Token(token) }
}

/// Like a [`Document`] where branch nodes
/// are nested maps and leaf nodes are typed values.
/// It is perhaps more akin to a filesystem where files are
/// typed, than a freeform json value.
#[derive(Default, Deref)]
pub struct TokenMap {
	tokens: HashMap<TokenKey, TokenValue>,
}
impl TokenMap {
	pub fn new() -> Self {
		Self {
			tokens: HashMap::new(),
		}
	}
	pub fn with(
		mut self,
		key: impl Into<TokenKey>,
		value: impl Into<TokenValue>,
	) -> Self {
		self.tokens.insert(key.into(), value.into());
		self
	}
}



#[macro_export]
macro_rules! token {
	(
		$(#[$meta:meta])*
		$new_ty:ident,
		$schema_ty:ident
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
		$schema_ty:ident,
		$doc_path: expr
	) => {
		#[derive(::bevy::reflect::TypePath)]
		$(#[$meta])*
		pub struct $new_ty;
		impl $crate::prelude::TypedToken for $new_ty {
			fn schema() -> $crate::prelude::TokenKey {
				$crate::prelude::TokenKey::of::<$schema_ty>()
			}
			fn key() -> $crate::prelude::TokenKey {
				$crate::prelude::TokenKey::of::<Self>()
			}
			fn token() -> $crate::prelude::Token {
				$crate::prelude::Token::new(Self::key(), Self::schema())
					.with_document($doc_path)
			}
		}
	};
}



#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn test_name() {
		// Name::type_info().type_path().xprintln();
		Token::of::<Name, Name>()
			.key()
			.to_string()
			.xpect_eq("io.crates/bevy_ecs/name/Name");

		Foo::key()
			.to_string()
			.xpect_eq("io.crates/beet_node/token/token/tests/Foo");
	}

	token!(
			/// Some cool type
			/// This now works perfectly!
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
