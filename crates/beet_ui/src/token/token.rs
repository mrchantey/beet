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
	/// Whether the value for this token should be searched for in parent contexts
	/// during RuleSet resolution, defaulting to true for regular tokens and false for property
	/// tokens.
	///
	/// Note that if a non-inherited token points to an inherited token,
	/// that token will be inherited. This is a common practice.
	///
	/// ```css
	/// // background-color is not inherited, but --primary is,
	/// // so may be inherited
	/// background-color: var(--primary);
	/// ```
	inherited: TokenInheritance,
}

#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TokenInheritance {
	#[default]
	Inherited,
	NotInherited,
}


impl Token {
	pub fn new(key: TokenKey, schema: TokenSchema) -> Self {
		Self {
			key,
			schema,
			inherited: default(),
		}
	}
	pub fn is_inherited(&self) -> bool {
		self.inherited == TokenInheritance::Inherited
	}

	#[track_caller]
	pub fn new_inline(schema: TokenSchema) -> Self {
		Self::new(TokenKey::new_inline(), schema)
	}
}


impl core::fmt::Display for Token {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		self.key.fmt(f)
	}
}

pub trait TypedToken: Into<Token> {
	type Value;
}


#[macro_export]
macro_rules! token {
	(
		$(#[$meta:meta])*
		$new_ty:ident,
		$schema_ty:ty
	) => {
		token!(
			$(#[$meta])*
			$new_ty,
			$schema_ty,
			Default::default()
		);
	};
	(
		$(#[$meta:meta])*
		$new_ty:ident,
		$schema_ty:ty,
		$inherited:expr
	) => {
		#[derive(::bevy::reflect::TypePath)]
		$(#[$meta])*
		pub struct $new_ty;
		impl $crate::prelude::TypedToken for $new_ty{
			type Value = $schema_ty;
		}
		impl Into<$crate::prelude::Token> for $new_ty {
			fn into(self) -> $crate::prelude::Token {
				$crate::prelude::Token::new(
					$crate::prelude::TokenKey::of::<Self>(),
					$crate::prelude::TokenSchema::of::<$schema_ty>(),
				).with_inherited($inherited)
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
	#[beet_core::test]
	fn test_name() {
		Foo.xinto::<Token>()
			.key()
			.to_string()
			.xpect_eq("io.crates/beet_ui/token/token/tests/Foo");
	}
}
