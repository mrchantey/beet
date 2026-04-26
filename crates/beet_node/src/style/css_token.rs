use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;




pub trait CssToken {
	fn selectors() -> Vec<Rule> { default() }
	fn declarations(
		builder: &CssBuilder,
		value: &TokenValue,
	) -> Result<Vec<(String, String)>>;
}


#[macro_export]
macro_rules! css_property {
 (
  $(#[$meta:meta])*
  $new_ty:ident,
  $schema_ty:ident,
  $doc_path: expr,
  $property: expr
 ) => {
  $crate::token!(
   $(#[$meta])*
   $new_ty,
   $schema_ty,
   $doc_path
  );
  impl $crate::prelude::style::CssToken for $new_ty {
   fn declarations(
    builder: &$crate::style::CssBuilder,
    value: &$crate::prelude::TokenValue,
   ) -> ::bevy::prelude::Result<Vec<(String, String)>> {
    Ok(vec![(
     $property.into(),
     builder.token_value_to_css::<$schema_ty>(value)?,
    )])
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
    builder: &$crate::style::CssBuilder,
    value: &$crate::prelude::TokenValue,
   ) -> ::bevy::prelude::Result<Vec<(String, String)>> {
    Ok(vec![(
     builder.css_key::<Self>()?,
     builder.token_value_to_css::<$schema_ty>(value)?,
    )])
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
