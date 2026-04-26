use crate::style::*;
use beet_core::prelude::*;




pub trait CssToken {
	fn properties() -> Vec<SmolStr> { default() }
	fn selectors() -> Vec<Rule> { default() }
	fn declarations(
		value: &Value,
		builder: &CssBuilder,
	) -> Result<Vec<(String, String)>>;
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
	use crate::prelude::*;
	token!(
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
