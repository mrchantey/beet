use std::sync::Arc;

use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;

pub trait AsCssToken {
	fn as_css_token(&self, value: &TokenValue) -> Result<CssToken>;
}

/// Store methods for looking up a schema path and resolving a value
#[derive(Default, Deref, Resource)]
pub struct CssTokenMap(
	HashMap<TokenKey, Arc<dyn 'static + Send + Sync + AsCssToken>>,
);
impl CssTokenMap {
	/// Registers a CSS value resolver keyed on `T::Tokens`'s type path.
	///
	/// Stored [`TypedValue`]s carry the schema of their *tokens* struct
	/// (the type actually passed to `with_value`), not the output type,
	/// so the key must match that tokens type.
	pub fn insert<T: 'static + Send + Sync + TypedTokenKey + AsCssToken>(
		mut self,
		token: T,
	) -> Self {
		self.0.insert(TokenKey::of::<T>(), Arc::new(token));
		self
	}
	pub fn get(
		&self,
		key: &TokenKey,
	) -> Result<&(dyn Send + Sync + AsCssToken)> {
		self.0.get(key).map(|arc| arc.as_ref()).ok_or_else(|| {
			bevyhow!("No CSS Token registered for this schema:\n{}", key)
		})
	}

	pub fn extend(&mut self, other: Self) -> &mut Self {
		self.0.extend(other.0);
		self
	}

	pub fn with_extend(mut self, other: Self) -> Self {
		self.0.extend(other.0);
		self
	}
}




#[macro_export]
macro_rules! css_property {
 (
  $(#[$meta:meta])*
  $new_ty:ident,
  $schema_ty:ident,
  $doc_path: expr,
  $($property: expr),+
 ) => {
  $crate::token!(
   $(#[$meta])*
   $new_ty,
   $schema_ty,
   $doc_path
  );
  impl $crate::prelude::style::AsCssToken for $new_ty {
   fn as_css_token(
    &self,
    value: &$crate::prelude::TokenValue,
   ) -> ::bevy::prelude::Result<$crate::prelude::style::CssToken> {
   	$crate::prelude::style::CssToken::from_props_value::<$schema_ty>(
   	vec![$($crate::prelude::style::CssKey::static_property($property)),+],
    value
   )
   }
  }
 };
}


#[macro_export]
macro_rules! css_variable {
 (
  $(#[$meta:meta])*
  $new_ty:ident,
  $schema_ty:ident,
  $doc_path: expr
 ) => {
  $crate::token!(
   $(#[$meta])*
   $new_ty,
   $schema_ty,
   $doc_path
  );
  impl $crate::prelude::style::AsCssToken for $new_ty {
   fn as_css_token(
   	&self,
    value: &$crate::prelude::TokenValue,
   ) -> ::bevy::prelude::Result<$crate::prelude::style::CssToken> {
   	$crate::prelude::style::CssToken::from_key_value::<$schema_ty>(&$new_ty::token_key(),value)
	 }
  }
 };
 (
  $(#[$meta:meta])*
  $new_ty:ident,
  $schema_ty:ident
 ) => {
  $crate::css_variable!(
   $(#[$meta])*
   $new_ty,
   $schema_ty,
   $crate::prelude::DocumentPath::default()
  );
 };
}

#[cfg(test)]
mod tests {
	use super::*;
	css_property!(
		#[allow(unused)]
		Foo,
		Color,
		DocumentPath::Ancestor,
		"color"
	);
	css_variable!(
		#[allow(unused)]
		Bar,
		Color,
		DocumentPath::Ancestor
	);

	#[test]
	fn test_name() {
		Bar.xinto::<Token>()
			.key()
			.to_string()
			.xpect_eq("io.crates/beet_node/style/css/css_token/tests/Bar");
	}
}
