use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;


#[derive(Reflect)]
pub struct ColorRole {
	pub background: Token,
	pub foreground: Token,
}

impl AsCssValues for ColorRole {
	fn properties() -> Vec<SmolStr> {
		vec![
			SmolStr::new_static("background-color"),
			SmolStr::new_static("color"),
		]
	}

	fn as_css_values(&self) -> Result<Vec<String>> {
		vec![
			CssIdent::from_token_key(self.background.key()).as_css_value(),
			CssIdent::from_token_key(self.foreground.key()).as_css_value(),
		]
		.xok()
	}
}
