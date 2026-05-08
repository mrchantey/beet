use crate::prelude::*;
use beet_core::prelude::*;

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


impl core::fmt::Display for Token {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		self.key.fmt(f)
	}
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
	#[allow(unused_imports)]
	use super::*;

	#[cfg(feature = "style")]
	token!(
		/// Some cool type
		#[derive(Debug, Clone)]
		Foo,
		Color
	);
	#[cfg(feature = "style")]
	token!(
		#[allow(unused)]
		Bar,
		Color
	);

	#[cfg(feature = "style")]
	#[test]
	fn test_name() {
		Foo.xinto::<Token>()
			.key()
			.to_string()
			.xpect_eq("io.crates/beet_ui/token/token/tests/Foo");
	}
}
