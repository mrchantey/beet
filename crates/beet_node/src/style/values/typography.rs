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

impl CssValue for Typeface {
	fn to_css_value(&self) -> String {
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
		out.join(", ")
	}
}

/// Font weight token, mapping semantic names to numeric CSS values.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect)]
pub enum FontWeight {
	Normal,
	Bold,
	Absolute(u16),
}

impl CssValue for FontWeight {
	fn to_css_value(&self) -> String {
		match self {
			Self::Normal => "normal".into(),
			Self::Bold => "bold".into(),
			Self::Absolute(val) => val.to_string(),
		}
	}
}

/// A complete typography style combining typeface, size, weight, and optional spacing.
#[derive(Debug, Clone, PartialEq, Reflect, FromTokens)]
pub struct Typography {
	/// [`FieldRef`] pointing to the [`Typeface`] token.
	#[token]
	pub typeface: Typeface,
	/// [`FieldRef`] pointing to the [`FontWeight`] token.
	#[token]
	pub weight: FontWeight,
	pub size: Length,
	pub line_height: Option<Length>,
	pub letter_spacing: Option<Length>,
}

impl CssValue for Typography {
	/// Returns CSS font properties as a space-separated shorthand:
	/// `"{weight} {size}/{line_height} {family}"`, omitting optional parts when absent.
	fn to_css_value(&self) -> String {
		let size = if let Some(lh) = &self.line_height {
			format!("{}/{}", self.size.to_css_value(), lh.to_css_value())
		} else {
			self.size.to_css_value()
		};
		let mut parts = vec![
			self.weight.to_css_value(),
			size,
			self.typeface.to_css_value(),
		];
		if let Some(ls) = &self.letter_spacing {
			parts.push(format!("letter-spacing:{}", ls.to_css_value()));
		}
		parts.join(" ")
	}
}
