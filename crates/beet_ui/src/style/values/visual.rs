//! CSS-value bridges for the upstreamed [`VisualStyle`] family.
//!
//! The types themselves live in [`beet_core`]; this module re-exports them and
//! adds the beet_ui-specific [`AsCssValue`]/[`AsCssValues`] implementations.
use crate::style::AsCssValue;
use crate::style::AsCssValues;
use crate::style::CssKey;
use crate::style::CssValue;
use beet_core::prelude::*;

pub use beet_core::prelude::DecorationLine;
pub use beet_core::prelude::DecorationStyle;
pub use beet_core::prelude::TextAlign;
pub use beet_core::prelude::TextStyle;
pub use beet_core::prelude::VISUAL_STYLE_DEFAULT;
pub use beet_core::prelude::VisualStyle;

impl AsCssValue for TextAlign {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::Left => "left",
			Self::Center => "center",
			Self::Right => "right",
		}
		.xmap(CssValue::expression)
		.xok()
	}
}

impl AsCssValue for DecorationLine {
	fn as_css_value(&self) -> Result<CssValue> {
		let mut parts = Vec::new();
		if self.underline {
			parts.push("underline");
		}
		if self.overline {
			parts.push("overline");
		}
		if self.line_through {
			parts.push("line-through");
		}
		if parts.is_empty() {
			return "none".xmap(CssValue::expression).xok();
		}
		let value = parts.join(" ");
		value.xmap(CssValue::expression).xok()
	}
}

impl AsCssValue for DecorationStyle {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::Solid => "solid",
			Self::Wavy => "wavy",
			Self::Double => "double",
			Self::Dash => "dashed",
		}
		.xmap(CssValue::expression)
		.xok()
	}
}

impl AsCssValues for TextStyle {
	fn suffixes() -> Vec<CssKey> {
		vec![
			CssKey::static_property("bold"),
			CssKey::static_property("italic"),
			CssKey::static_property("blink"),
			CssKey::static_property("rapidBlink"),
			CssKey::static_property("reversed"),
			CssKey::static_property("hidden"),
		]
	}

	fn as_css_values(&self) -> Result<Vec<CssValue>> {
		// TODO this is very incorrect,
		// the type probs needs to be reworked, split
		// based on css property mappings
		let mut values = Vec::new();
		if self.contains(Self::BOLD) {
			values.push(CssValue::expression("bold"));
		}
		if self.contains(Self::ITALIC) {
			values.push(CssValue::expression("italic"));
		}
		if self.contains(Self::BLINK) {
			values.push(CssValue::expression("blink"));
		}
		if self.contains(Self::RAPID_BLINK) {
			values.push(CssValue::expression("rapid-blink"));
		}
		if self.contains(Self::REVERSED) {
			values.push(CssValue::expression("reversed"));
		}
		if self.contains(Self::HIDDEN) {
			values.push(CssValue::expression("hidden"));
		}
		if values.is_empty() {
			values.push(CssValue::expression("normal"));
		}
		values.xok()
	}
}
