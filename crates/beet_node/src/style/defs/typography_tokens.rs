#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::style::*;
use crate::token;

// ── Ref tokens: typeface families ────────────────────────────────────────────

token!(Typeface, TYPEFACE_BRAND, "typeface-brand");
token!(Typeface, TYPEFACE_PLAIN, "typeface-plain");
// Monospace family for code and pre elements.
token!(Typeface, TYPEFACE_MONO,  "typeface-mono");

// ── Ref tokens: font weights ──────────────────────────────────────────────────

token!(FontWeight, WEIGHT_REGULAR, "weight-regular");
token!(FontWeight, WEIGHT_MEDIUM,  "weight-medium");
token!(FontWeight, WEIGHT_BOLD,    "weight-bold");

// ── Ref tokens: font sizes (MD3 type scale) ───────────────────────────────────

token!(Length, FONT_SIZE_DISPLAY_LARGE,   "font-size-display-large");
token!(Length, FONT_SIZE_DISPLAY_MEDIUM,  "font-size-display-medium");
token!(Length, FONT_SIZE_DISPLAY_SMALL,   "font-size-display-small");
token!(Length, FONT_SIZE_HEADLINE_LARGE,  "font-size-headline-large");
token!(Length, FONT_SIZE_HEADLINE_MEDIUM, "font-size-headline-medium");
token!(Length, FONT_SIZE_HEADLINE_SMALL,  "font-size-headline-small");
token!(Length, FONT_SIZE_TITLE_LARGE,     "font-size-title-large");
token!(Length, FONT_SIZE_TITLE_MEDIUM,    "font-size-title-medium");
token!(Length, FONT_SIZE_TITLE_SMALL,     "font-size-title-small");
token!(Length, FONT_SIZE_BODY_LARGE,      "font-size-body-large");
token!(Length, FONT_SIZE_BODY_MEDIUM,     "font-size-body-medium");
token!(Length, FONT_SIZE_BODY_SMALL,      "font-size-body-small");
token!(Length, FONT_SIZE_LABEL_LARGE,     "font-size-label-large");
token!(Length, FONT_SIZE_LABEL_MEDIUM,    "font-size-label-medium");
token!(Length, FONT_SIZE_LABEL_SMALL,     "font-size-label-small");

// ── Ref tokens: line heights (MD3 type scale) ─────────────────────────────────

token!(Length, LINE_HEIGHT_DISPLAY_LARGE,   "line-height-display-large");
token!(Length, LINE_HEIGHT_DISPLAY_MEDIUM,  "line-height-display-medium");
token!(Length, LINE_HEIGHT_DISPLAY_SMALL,   "line-height-display-small");
token!(Length, LINE_HEIGHT_HEADLINE_LARGE,  "line-height-headline-large");
token!(Length, LINE_HEIGHT_HEADLINE_MEDIUM, "line-height-headline-medium");
token!(Length, LINE_HEIGHT_HEADLINE_SMALL,  "line-height-headline-small");
token!(Length, LINE_HEIGHT_TITLE_LARGE,     "line-height-title-large");
token!(Length, LINE_HEIGHT_TITLE_MEDIUM,    "line-height-title-medium");
token!(Length, LINE_HEIGHT_TITLE_SMALL,     "line-height-title-small");
token!(Length, LINE_HEIGHT_BODY_LARGE,      "line-height-body-large");
token!(Length, LINE_HEIGHT_BODY_MEDIUM,     "line-height-body-medium");
token!(Length, LINE_HEIGHT_BODY_SMALL,      "line-height-body-small");
token!(Length, LINE_HEIGHT_LABEL_LARGE,     "line-height-label-large");
token!(Length, LINE_HEIGHT_LABEL_MEDIUM,    "line-height-label-medium");
token!(Length, LINE_HEIGHT_LABEL_SMALL,     "line-height-label-small");

// ── Sys tokens: composite typography scales ───────────────────────────────────

token!(Typography, DISPLAY_LARGE,   "display-large");
token!(Typography, DISPLAY_MEDIUM,  "display-medium");
token!(Typography, DISPLAY_SMALL,   "display-small");
token!(Typography, HEADLINE_LARGE,  "headline-large");
token!(Typography, HEADLINE_MEDIUM, "headline-medium");
token!(Typography, HEADLINE_SMALL,  "headline-small");
token!(Typography, TITLE_LARGE,     "title-large");
token!(Typography, TITLE_MEDIUM,    "title-medium");
token!(Typography, TITLE_SMALL,     "title-small");
token!(Typography, BODY_LARGE,      "body-large");
token!(Typography, BODY_MEDIUM,     "body-medium");
token!(Typography, BODY_SMALL,      "body-small");
token!(Typography, LABEL_LARGE,     "label-large");
token!(Typography, LABEL_MEDIUM,    "label-medium");
token!(Typography, LABEL_SMALL,     "label-small");


/// Returns a [`TokenStore`] with all MD3 typography tokens.
///
/// Includes ref tokens (typefaces, weights, font sizes, line heights)
/// and sys tokens (the 15 composite typescale entries).
pub fn default_typography() -> TokenStore {
	// Common typeface stacks
	let brand = Typeface::new(["Google Sans", "Product Sans", "Inter", "Work Sans", "system-ui", "sans-serif"]);
	let plain = Typeface::new(["Roboto", "system-ui", "-apple-system", "BlinkMacSystemFont", "Segoe UI", "sans-serif"]);
	let mono  = Typeface::new(["Roboto Mono", "'Courier New'", "monospace"]);

	TokenStore::new()
		// ── Typeface ref tokens ───────────────────────────────────────────────
		.with(TYPEFACE_PLAIN,  plain)
		.with(TYPEFACE_BRAND,  brand)
		.with(TYPEFACE_MONO,   mono)
		// ── Weight ref tokens ─────────────────────────────────────────────────
		.with(WEIGHT_REGULAR,  FontWeight::Absolute(400))
		.with(WEIGHT_MEDIUM,   FontWeight::Absolute(500))
		.with(WEIGHT_BOLD,     FontWeight::Absolute(700))
		// ── Font size ref tokens (MD3 sp → rem at 16 px base) ─────────────────
		.with(FONT_SIZE_DISPLAY_LARGE,   Length::rem(3.5625))
		.with(FONT_SIZE_DISPLAY_MEDIUM,  Length::rem(2.8125))
		.with(FONT_SIZE_DISPLAY_SMALL,   Length::rem(2.25))
		.with(FONT_SIZE_HEADLINE_LARGE,  Length::rem(2.0))
		.with(FONT_SIZE_HEADLINE_MEDIUM, Length::rem(1.75))
		.with(FONT_SIZE_HEADLINE_SMALL,  Length::rem(1.5))
		.with(FONT_SIZE_TITLE_LARGE,     Length::rem(1.375))
		.with(FONT_SIZE_TITLE_MEDIUM,    Length::rem(1.0))
		.with(FONT_SIZE_TITLE_SMALL,     Length::rem(0.875))
		.with(FONT_SIZE_BODY_LARGE,      Length::rem(1.0))
		.with(FONT_SIZE_BODY_MEDIUM,     Length::rem(0.875))
		.with(FONT_SIZE_BODY_SMALL,      Length::rem(0.75))
		.with(FONT_SIZE_LABEL_LARGE,     Length::rem(0.875))
		.with(FONT_SIZE_LABEL_MEDIUM,    Length::rem(0.75))
		.with(FONT_SIZE_LABEL_SMALL,     Length::rem(0.6875))
		// ── Line height ref tokens (MD3 sp → rem at 16 px base) ───────────────
		.with(LINE_HEIGHT_DISPLAY_LARGE,   Length::rem(4.0))
		.with(LINE_HEIGHT_DISPLAY_MEDIUM,  Length::rem(3.25))
		.with(LINE_HEIGHT_DISPLAY_SMALL,   Length::rem(2.75))
		.with(LINE_HEIGHT_HEADLINE_LARGE,  Length::rem(2.5))
		.with(LINE_HEIGHT_HEADLINE_MEDIUM, Length::rem(2.25))
		.with(LINE_HEIGHT_HEADLINE_SMALL,  Length::rem(2.0))
		.with(LINE_HEIGHT_TITLE_LARGE,     Length::rem(1.75))
		.with(LINE_HEIGHT_TITLE_MEDIUM,    Length::rem(1.5))
		.with(LINE_HEIGHT_TITLE_SMALL,     Length::rem(1.25))
		.with(LINE_HEIGHT_BODY_LARGE,      Length::rem(1.5))
		.with(LINE_HEIGHT_BODY_MEDIUM,     Length::rem(1.25))
		.with(LINE_HEIGHT_BODY_SMALL,      Length::rem(1.0))
		.with(LINE_HEIGHT_LABEL_LARGE,     Length::rem(1.25))
		.with(LINE_HEIGHT_LABEL_MEDIUM,    Length::rem(1.0))
		.with(LINE_HEIGHT_LABEL_SMALL,     Length::rem(1.0))
		// ── Composite typography sys tokens ───────────────────────────────────
		.with(DISPLAY_LARGE,   Typography { typeface: TYPEFACE_BRAND, weight: WEIGHT_REGULAR, size: Length::rem(3.5625), line_height: None, letter_spacing: None })
		.with(DISPLAY_MEDIUM,  Typography { typeface: TYPEFACE_BRAND, weight: WEIGHT_REGULAR, size: Length::rem(2.8125), line_height: None, letter_spacing: None })
		.with(DISPLAY_SMALL,   Typography { typeface: TYPEFACE_BRAND, weight: WEIGHT_REGULAR, size: Length::rem(2.25),   line_height: None, letter_spacing: None })
		.with(HEADLINE_LARGE,  Typography { typeface: TYPEFACE_PLAIN, weight: WEIGHT_REGULAR, size: Length::rem(2.0),    line_height: None, letter_spacing: None })
		.with(HEADLINE_MEDIUM, Typography { typeface: TYPEFACE_PLAIN, weight: WEIGHT_REGULAR, size: Length::rem(1.75),   line_height: None, letter_spacing: None })
		.with(HEADLINE_SMALL,  Typography { typeface: TYPEFACE_PLAIN, weight: WEIGHT_REGULAR, size: Length::rem(1.5),    line_height: None, letter_spacing: None })
		.with(TITLE_LARGE,     Typography { typeface: TYPEFACE_PLAIN, weight: WEIGHT_REGULAR, size: Length::rem(1.375),  line_height: None, letter_spacing: None })
		.with(TITLE_MEDIUM,    Typography { typeface: TYPEFACE_PLAIN, weight: WEIGHT_MEDIUM,  size: Length::rem(1.0),    line_height: None, letter_spacing: None })
		.with(TITLE_SMALL,     Typography { typeface: TYPEFACE_PLAIN, weight: WEIGHT_MEDIUM,  size: Length::rem(0.875),  line_height: None, letter_spacing: None })
		.with(BODY_LARGE,      Typography { typeface: TYPEFACE_PLAIN, weight: WEIGHT_REGULAR, size: Length::rem(1.0),    line_height: None, letter_spacing: None })
		.with(BODY_MEDIUM,     Typography { typeface: TYPEFACE_PLAIN, weight: WEIGHT_REGULAR, size: Length::rem(0.875),  line_height: None, letter_spacing: None })
		.with(BODY_SMALL,      Typography { typeface: TYPEFACE_PLAIN, weight: WEIGHT_REGULAR, size: Length::rem(0.75),   line_height: None, letter_spacing: None })
		.with(LABEL_LARGE,     Typography { typeface: TYPEFACE_PLAIN, weight: WEIGHT_MEDIUM,  size: Length::rem(0.875),  line_height: None, letter_spacing: None })
		.with(LABEL_MEDIUM,    Typography { typeface: TYPEFACE_PLAIN, weight: WEIGHT_MEDIUM,  size: Length::rem(0.75),   line_height: None, letter_spacing: None })
		.with(LABEL_SMALL,     Typography { typeface: TYPEFACE_PLAIN, weight: WEIGHT_MEDIUM,  size: Length::rem(0.6875), line_height: None, letter_spacing: None })
}
