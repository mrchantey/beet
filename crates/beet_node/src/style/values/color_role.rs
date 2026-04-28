use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;

css_property!(
	ColorRoleProps,
	ColorRole,
	DocumentPath::Ancestor,
	"background-color",
	"color"
);


#[derive(Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ColorRole {
	pub background: Token,
	pub foreground: Token,
}

impl AsCssValues for ColorRole {
	fn suffixes() -> Vec<CssKey> {
		vec![CssKey::static_property("bg"), CssKey::static_property("fg")]
	}

	fn as_css_values(&self) -> Result<Vec<CssValue>> {
		vec![
			CssVariable::from_token_key(self.background.key())
				.xinto::<CssValue>(),
			CssVariable::from_token_key(self.foreground.key())
				.xinto::<CssValue>(),
		]
		.xok()
	}
}
