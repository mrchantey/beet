use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;
use std::sync::LazyLock;

/// The typeface family list to use, with the first match selected
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deref, Reflect)]
pub struct Typeface(Vec<SmolStr>);

impl Typeface {
	pub fn new<T: Into<SmolStr>>(
		families: impl IntoIterator<Item = T>,
	) -> Self {
		Self(families.into_iter().map(Into::into).collect())
	}

	pub fn iter(&self) -> impl Iterator<Item = &SmolStr> { self.0.iter() }
}

impl AsCssValue for Typeface {
	fn as_css_value(&self) -> Result<String> {
		// system font keywords must not be quoted
		const SYSTEM_FONT_KEYWORDS: LazyLock<HashSet<&'static str>> =
			LazyLock::new(|| {
				HashSet::from([
					// Official W3C Standard Keywords
					"system-ui",
					"ui-sans-serif",
					"ui-serif",
					"ui-monospace",
					"ui-rounded",
					"sans-serif",
					"serif",
					"monospace",
					"cursive",
					"fantasy",
					"emoji",
					"math",
					"fangsong",
					"nastaliq",
					// Vendor-Specific / Legacy Prefixed Keywords
					"-apple-system",      // Safari/iOS legacy
					"BlinkMacSystemFont", // Chrome on macOS legacy
					// Global CSS Identifiers
					"inherit",
					"initial",
					"unset",
					"revert",
					"revert-layer",
				])
			});

		let mut out = Vec::with_capacity(self.len());

		for family in self.iter() {
			if family.starts_with("'")
				|| family.starts_with("\"")
				|| SYSTEM_FONT_KEYWORDS.contains(family.as_str())
			{
				out.push(family.to_string());
			} else {
				out.push(format!("\"{}\"", family));
			}
		}
		out.join(", ").xok()
	}
}

/// Font weight token, mapping semantic names to numeric CSS values.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect)]
pub enum FontWeight {
	Normal,
	Bold,
	Absolute(u16),
}

impl AsCssValue for FontWeight {
	fn as_css_value(&self) -> Result<String> {
		match self {
			Self::Normal => "normal".into(),
			Self::Bold => "bold".into(),
			Self::Absolute(val) => val.to_string(),
		}
		.xok()
	}
}

/// A complete typography style combining typeface, size, weight, and optional spacing.
#[derive(Debug, Clone, PartialEq, Reflect)]
pub struct Typography {
	/// [`FieldRef`] pointing to the [`Typeface`] token.
	pub typeface: Token,
	/// [`FieldRef`] pointing to the [`FontWeight`] token.
	pub weight: Token,
	pub size: Length,
	pub line_height: Option<Length>,
	pub letter_spacing: Option<Length>,
}


impl AsCssValue for Typography {
	/// Returns CSS font properties as a space-separated shorthand:
	/// `"{weight} {size}/{line_height} {family}"`, omitting optional parts when absent.
	fn as_css_value(&self) -> Result<String> {
		let size = if let Some(lh) = &self.line_height {
			format!(
				"{}/{}",
				self.size.as_css_values()?[0],
				lh.as_css_values()?[0]
			)
		} else {
			self.size.as_css_values()?[0].clone()
		};
		let mut parts = vec![
			CssIdent::from_token_key(self.weight.key()).as_css_value(),
			// builder.ide
			// builder.iden
			// builder.self.weight.to_css_value(builder),
			size,
			CssIdent::from_token_key(self.typeface.key()).as_css_value(),
		];
		if let Some(ls) = &self.letter_spacing {
			parts.push(format!("letter-spacing:{}", ls.as_css_values()?[0]));
		}
		parts.join(" ").xok()
	}
}
