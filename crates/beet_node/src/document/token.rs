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
	path: TokenPath,
	/// The path to the document
	document: DocumentPath,
	/// Path to the token representing the type of this token,
	/// ie `io.crates/bevy_math/Color`
	schema: TokenPath,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, Get)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TypedValue {
	value: Value,
	/// Path to the token representing the type of this token,
	/// ie `io.crates/bevy_math/Color`
	schema: TokenPath,
}

impl TypedValue {
	pub fn new<T: Typed>(value: T) -> Result<Self> {
		Self {
			value: Value::from_reflect(&value)?,
			schema: TokenPath::of::<T>(),
		}
		.xok()
	}
}


impl Token {
	pub fn new(path: TokenPath, schema: TokenPath) -> Self {
		Self {
			path,
			schema,
			document: default(),
		}
	}

	/// Create new token, using `Token` for the field path
	pub fn of<Path: TypePath, Schema: TypePath>() -> Self {
		Self {
			path: TokenPath::of::<Path>(),
			schema: TokenPath::of::<Schema>(),
			document: default(),
		}
	}
}

/// A type which represents a token, see `token2!` for defining.
pub trait TypedToken {
	fn schema() -> TokenPath;
	fn path() -> TokenPath;
	fn token() -> Token {
		Token {
			path: Self::path(),
			schema: Self::schema(),
			document: default(),
		}
	}
}

impl<T: TypedToken> From<T> for Token {
	fn from(_: T) -> Self { T::token() }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ValueOrToken {
	Value(TypedValue),
	Token(Token),
}

impl From<TypedValue> for ValueOrToken {
	fn from(value: TypedValue) -> Self { Self::Value(value) }
}
impl From<Token> for ValueOrToken {
	fn from(token: Token) -> Self { Self::Token(token) }
}

/// Like a [`Value`] where branch nodes
/// are nested maps and leaf nodes are typed values.
/// It is perhaps more akin to a filesystem where files are
/// typed, than a freeform json value.
#[derive(Default, Deref)]
pub struct DynamicDocument {
	tokens: HashMap<TokenPath, Token>,
}
impl DynamicDocument {
	pub fn new() -> Self {
		Self {
			tokens: HashMap::new(),
		}
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
			fn schema() -> $crate::prelude::TokenPath {
				$crate::prelude::TokenPath::of::<$schema_ty>()
			}
			fn path() -> $crate::prelude::TokenPath {
				let path = ::core::concat!(
					::core::concat!(::core::module_path!(), "::"),
					::core::stringify!($new_ty)
				);
				$crate::prelude::TokenPath::from_module_path(path)
			}
			fn token() -> $crate::prelude::Token {
				$crate::prelude::Token::new(Self::path(),Self::schema())
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
			.path()
			.to_string()
			.xpect_eq("io.crates/bevy_ecs/name/Name");

		Foo::path()
			.to_string()
			.xpect_eq("io.crates/beet_node/document/token/tests/Foo");
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
