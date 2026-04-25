use crate::prelude::FieldRef;
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

/// A complete typography token combining typeface, size, and weight
/// into a CSS font shorthand.
#[derive(Debug, Clone, PartialEq, Reflect)]
pub struct Typography {
	/// [`FieldRef`] pointing to the [`Typeface`] token.
	pub typeface: FieldRef,
	/// [`FieldRef`] pointing to the [`FontWeight`] token.
	pub weight: FieldRef,
	pub size: Length,
	pub line_height: Option<Length>,
	pub letter_spacing: Option<Length>,
}

impl CssValue for Typography {
	/// Returns a CSS font shorthand: `"{weight} {size} {family}"`.
	/// Requires resolving typeface and weight [`FieldRef`]s via a token store.
	fn to_css_value(&self) -> String {
		todo!("resolve typeface and weight FieldRefs via token store")
	}
}
