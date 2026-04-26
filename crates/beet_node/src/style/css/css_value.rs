use crate::style::*;
use beet_core::prelude::*;

/// Converts a value to its CSS string representation.
pub trait AsCssValues {
	/// If the type uses multiple properties declare them here.
	fn properties() -> Vec<SmolStr> { default() }
	fn as_css_values(&self, builder: &CssBuilder) -> Result<Vec<String>>;
}

#[derive(Debug, Clone)]
pub enum CssIdent {
	/// The variable name without a leading `--`
	Variable(SmolStr),
	Property(SmolStr),
}

impl CssIdent {
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


impl AsCssValues for Color {
	fn as_css_values(&self, _builder: &CssBuilder) -> Result<Vec<String>> {
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
		.xvec()
		.xok()
	}
}
