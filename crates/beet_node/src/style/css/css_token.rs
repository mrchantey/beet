use std::sync::Arc;

use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;

pub trait CssToken {
	fn as_css_rule(&self, value: &TokenValue) -> Result<CssRule>;
}

/// Store methods for looking up a schema path and resolving a value
#[derive(Default, Deref, Resource)]
pub struct CssTokenMap(
	HashMap<TokenKey, Arc<dyn 'static + Send + Sync + CssToken>>,
);
impl CssTokenMap {
	/// Registers a CSS value resolver keyed on `T::Tokens`'s type path.
	///
	/// Stored [`TypedValue`]s carry the schema of their *tokens* struct
	/// (the type actually passed to `with_value`), not the output type,
	/// so the key must match that tokens type.
	pub fn insert<T: 'static + Send + Sync + TypedTokenKey + CssToken>(
		mut self,
		token: T,
	) -> Self {
		self.0.insert(TokenKey::of::<T>(), Arc::new(token));
		self
	}
	pub fn get(&self, key: &TokenKey) -> Result<&(dyn Send + Sync + CssToken)> {
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
  impl $crate::prelude::style::CssToken for $new_ty {
   fn as_css_rule(
    &self,
    value: &$crate::prelude::TokenValue,
   ) -> ::bevy::prelude::Result<$crate::prelude::style::CssRule> {
   	$crate::prelude::style::CssRule::from_props_value::<$schema_ty>(
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
  impl $crate::prelude::style::CssToken for $new_ty {
   fn as_css_rule(
   	&self,
    value: &$crate::prelude::TokenValue,
   ) -> ::bevy::prelude::Result<$crate::prelude::style::CssRule> {
   	$crate::prelude::style::CssRule::from_key_value::<$new_ty,$schema_ty>(value)
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
		// Name::type_info().type_path().xprintln();
		Token::of::<Name, Name>()
			.key()
			.to_string()
			.xpect_eq("io.crates/bevy_ecs/name/Name");

		Bar::key()
			.to_string()
			.xpect_eq("io.crates/beet_node/style/css/css_token/tests/Bar");
	}
}
