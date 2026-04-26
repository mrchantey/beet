use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;

css_property!(ColorRoleProps, ColorRole, DocumentPath::Ancestor);

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

	fn as_css_values(&self, builder: &CssBuilder) -> Result<Vec<String>> {
		vec![
			builder.ident_to_css(self.background.key())?.as_css_value(),
			builder.ident_to_css(self.foreground.key())?.as_css_value(),
		]
		.xok()
	}
}
