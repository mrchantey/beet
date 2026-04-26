use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;



#[derive(Reflect)]
pub struct ColorRole {
	pub background: Token,
	pub foreground: Token,
}

impl AsCssValues for ColorRole {
	fn as_css_values(&self, builder: &CssBuilder) -> Result<Vec<String>> {
		vec![
			builder.ident_to_css(self.background.key())?.as_css_value(),
			builder.ident_to_css(self.foreground.key())?.as_css_value(),
		]
		.xok()
	}
}

#[derive(Reflect)]
pub struct ColorRoleProps;

impl CssToken for ColorRoleProps {
	fn declarations(
		&self,
		builder: &CssBuilder,
		value: &TokenValue,
	) -> ::bevy::prelude::Result<Vec<(String, String)>> {
		let values = builder.token_value_to_css::<ColorRole>(value)?;
		vec!["background-color".to_string(), "color".to_string()]
			.into_iter()
			.zip(values)
			.collect::<Vec<_>>()
			.xok()
	}
}
