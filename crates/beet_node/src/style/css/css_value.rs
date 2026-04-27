use crate::style::*;
use beet_core::prelude::*;

/// Converts a value to its CSS string representation.
pub trait AsCssValues {
	fn suffixes() -> Vec<CssKey>;
	fn as_css_values(&self) -> Result<Vec<CssValue>>;
}

pub trait AsCssValue {
	fn property() -> Option<CssKey> { None }
	fn as_css_value(&self) -> Result<CssValue>;
}

impl<T: AsCssValue> AsCssValues for T {
	fn suffixes() -> Vec<CssKey> {
		if let Some(prop) = T::property() {
			vec![prop]
		} else {
			default()
		}
	}
	fn as_css_values(&self) -> Result<Vec<CssValue>> {
		self.as_css_value().map(|val| val.xvec())
	}
}

impl AsCssValue for Color {
	fn as_css_value(&self) -> Result<CssValue> {
		let this = self.to_srgba();
		let alpha = this.alpha;
		// still undecided about this..
		// what if user wants to overwrite
		if alpha == 1.0 {
			format!(
				"rgb({}, {}, {})",
				(this.red * 255.0).round() as u8,
				(this.green * 255.0).round() as u8,
				(this.blue * 255.0).round() as u8,
			)
		} else {
			format!(
				"rgba({}, {}, {}, {})",
				(this.red * 255.0).round() as u8,
				(this.green * 255.0).round() as u8,
				(this.blue * 255.0).round() as u8,
				alpha
			)
		}
		.xmap(CssValue::expression)
		.xok()
	}
}
