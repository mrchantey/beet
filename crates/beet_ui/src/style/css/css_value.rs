use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;

/// The right hand side of a css declaration
#[derive(Debug, Clone)]
pub enum CssValue {
	/// A variable, ie `var(--color-primary)`
	Variable(CssVariable),
	/// A raw expression, ie `rgb(0,0,0)`
	Expression(String),
}

impl CssValue {
	pub fn expression(value: impl Into<String>) -> Self {
		Self::Expression(value.into())
	}

	pub fn from_token_value<
		V: 'static
			+ Send
			+ Sync
			+ DeserializeOwned
			+ Typed
			+ TypedTokenKey
			+ AsCssValues,
	>(
		value: &TokenValue,
	) -> Result<Vec<Self>> {
		value.schema().assert_eq_ty::<V>()?;
		match value {
			TokenValue::Value(value) => {
				value.value().clone().into_serde::<V>()?.as_css_values()
			}
			TokenValue::Token(token) => Self::from_token::<V>(token).xok(),
		}
	}
	/// Represent tokens as css values, appending the property names in
	/// the case there are multiple
	pub fn from_token<T: AsCssValues>(token: &Token) -> Vec<Self> {
		let var = CssVariable::from_token_key(token.key());
		let suffixes = T::suffixes();
		if suffixes.len() <= 1 {
			// no need for suffix for no declared props
			var.xinto::<Self>().xvec()
		} else {
			suffixes
				.into_iter()
				.map(|suffix| {
					var.with_suffix(suffix.to_string()).xinto::<Self>()
				})
				.collect::<Vec<_>>()
		}
	}
}

impl std::fmt::Display for CssValue {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			CssValue::Variable(var) => var.as_css_value().fmt(f),
			CssValue::Expression(expr) => expr.fmt(f),
		}
	}
}

impl From<CssVariable> for CssValue {
	fn from(var: CssVariable) -> Self { Self::Variable(var) }
}


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
