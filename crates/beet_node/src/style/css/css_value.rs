use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;

/// Converts a value to its CSS string representation.
pub trait AsCssValues {
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

/// A token key is always represented as a css variable
/// when in the css value position, ie:
/// `foo: var(--path-to-this-token)`
impl AsCssValues for Token {
	fn as_css_values(&self, builder: &CssBuilder) -> Result<Vec<String>> {
		builder
			.ident_to_css(self.key())?
			.as_css_value()
			.xvec()
			.xok()
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
