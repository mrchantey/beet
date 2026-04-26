use std::sync::Arc;

use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;

pub trait CssToken {
	fn selectors(&self) -> Vec<Rule> { default() }
	fn declarations(
		&self,
		builder: &CssBuilder,
		value: &TokenValue,
	) -> Result<Vec<(String, String)>>;
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

	pub fn merge(mut self, other: Self) -> Self {
		self.0.extend(other.0);
		self
	}

	pub fn resolve(
		&self,
		builder: &CssBuilder,
		key: &TokenKey,
		value: &TokenValue,
	) -> Result<Vec<(String, String)>> {
		if let Some(token) = self.0.get(key) {
			// if let Some(func) = self.0.get(value.schema()) {
			token.declarations(builder, value)
		} else {
			bevybail!("No CSS Token registered for this schema:\n{}", key)
		}
	}
}




#[macro_export]
macro_rules! css_property {
	// collects props via $schema_ty::properties()
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
   fn declarations(
    &self,
    builder: &$crate::style::CssBuilder,
    value: &$crate::prelude::TokenValue,
   ) -> ::bevy::prelude::Result<Vec<(String, String)>> {
   builder.props_and_value_to_css::<$schema_ty>(
   	$schema_ty::properties(),
    value
   )
   }
  }
 };
	// properties are hardcoded
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
   fn declarations(
    &self,
    builder: &$crate::style::CssBuilder,
    value: &$crate::prelude::TokenValue,
   ) -> ::bevy::prelude::Result<Vec<(String, String)>> {
   builder.props_and_value_to_css::<$schema_ty>(
   	vec![$($property.to_string()),+],
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
   fn declarations(
   	&self,
    builder: &$crate::style::CssBuilder,
    value: &$crate::prelude::TokenValue,
   ) -> ::bevy::prelude::Result<Vec<(String, String)>> {
    	builder.key_value_to_css::<$new_ty,$schema_ty>(value)
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





#[macro_export]
macro_rules! css_token {
	(
		$(#[$meta:meta])*
		$new_ty:ident,
		$schema_ty:ident,
		$doc_path: expr,
		[$(props:expr),*]
	) => {
		$crate::token!(
			$(#[$meta])
			*$new_ty,
			$schema_ty,
			$doc_path
		)
		impl $crate::prelude::CssToken for $new_ty{
			fn properties() -> Vec<SmolStr> {
				vec![$(SmolStr::from($props)),*]
			}
		}
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
			.xpect_eq("io.crates/beet_node/document/token/tests/Foo");
	}
}
