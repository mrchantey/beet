use crate::token::TokenKey;
use beet_core::prelude::*;

/// Converts a value to its CSS string representation.
pub trait AsCssValues {
	/// If the type uses multiple properties declare them here.
	fn properties() -> Vec<SmolStr> { default() }
	fn as_css_values(&self) -> Result<Vec<String>>;
}

pub trait AsCssValue {
	fn property() -> Option<SmolStr> { None }
	fn as_css_value(&self) -> Result<String>;
}

impl<T: AsCssValue> AsCssValues for T {
	fn properties() -> Vec<SmolStr> {
		if let Some(prop) = T::property() {
			vec![prop]
		} else {
			default()
		}
	}
	fn as_css_values(&self) -> Result<Vec<String>> {
		self.as_css_value().map(|val| val.xvec())
	}
}

#[derive(Debug, Clone)]
pub enum CssIdent {
	/// The variable name without a leading `--`
	Variable(SmolStr),
	Property(SmolStr),
}

impl CssIdent {
	/// Returns the ident in css form, using the [`CssIdentMap`]
	/// if a mapping is found, otherwise the last part of
	/// the field path as a variable.
	/// Non-specified idents are assumed to be variables, not properties.
	pub fn from_token_key(token_key: &TokenKey) -> Self {
		use heck::ToKebabCase;
		let token_key =
			token_key.to_string().to_kebab_case().replace("/", "--");
		CssIdent::variable(token_key)
	}


	pub fn variable(name: impl Into<SmolStr>) -> Self {
		Self::Variable(name.into())
	}
	pub fn property(name: impl Into<SmolStr>) -> Self {
		Self::Property(name.into())
	}

	pub fn with_suffix(&self, suffix: impl Into<SmolStr>) -> Self {
		match self {
			CssIdent::Variable(var) => {
				CssIdent::Variable(format!("{}-{}", var, suffix.into()).into())
			}
			CssIdent::Property(prop) => {
				CssIdent::Property(format!("{}-{}", prop, suffix.into()).into())
			}
		}
	}

	pub fn as_css_key(&self) -> String { self.to_string() }
	pub fn as_css_value(&self) -> String {
		match self {
			CssIdent::Variable(var) => format!("var(--{})", var),
			CssIdent::Property(prop) => prop.to_string(),
		}
	}
}

impl std::fmt::Display for CssIdent {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			CssIdent::Variable(var) => write!(f, "--{}", var),
			CssIdent::Property(prop) => write!(f, "{}", prop),
		}
	}
}


impl AsCssValue for Color {
	fn as_css_value(&self) -> Result<String> {
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
		.xok()
	}
}
