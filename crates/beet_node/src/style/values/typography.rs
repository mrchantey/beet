use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;
use std::sync::LazyLock;

css_property!(
	TypographyProps,
	Typography,
	DocumentPath::Ancestor,
	"font-family",
	"font-weight",
	"font-size",
	"line-height",
	"letter-spacing"
);

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
	fn as_css_value(&self) -> Result<CssValue> {
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
		out.join(", ").xmap(CssValue::expression).xok()
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
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::Normal => "normal".into(),
			Self::Bold => "bold".into(),
			Self::Absolute(val) => val.to_string(),
		}
		.xmap(CssValue::expression)
		.xok()
	}
}

/// A complete typography style combining typeface, size, weight, line height, and letter spacing.
#[derive(Debug, Clone, PartialEq, Reflect)]
pub struct Typography {
	/// [`Token`] pointing to the [`Typeface`] token.
	pub typeface: Token,
	/// [`Token`] pointing to the [`FontWeight`] token.
	pub weight: Token,
	/// [`Token`] pointing to the font size [`Length`] token.
	pub size: Token,
	/// [`Token`] pointing to the line height [`Length`] token.
	pub line_height: Token,
	/// [`Token`] pointing to the letter spacing [`Length`] token.
	pub letter_spacing: Token,
}

impl AsCssValues for Typography {
	fn suffixes() -> Vec<CssKey> {
		vec![
			CssKey::static_property("family"),
			CssKey::static_property("weight"),
			CssKey::static_property("size"),
			CssKey::static_property("lh"),
			CssKey::static_property("ls"),
		]
	}

	fn as_css_values(&self) -> Result<Vec<CssValue>> {
		vec![
			CssVariable::from_token_key(self.typeface.key())
				.xinto::<CssValue>(),
			CssVariable::from_token_key(self.weight.key()).xinto::<CssValue>(),
			CssVariable::from_token_key(self.size.key()).xinto::<CssValue>(),
			CssVariable::from_token_key(self.line_height.key())
				.xinto::<CssValue>(),
			CssVariable::from_token_key(self.letter_spacing.key())
				.xinto::<CssValue>(),
		]
		.xok()
	}
}
