#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::*;


// ── Typeface ref tokens ───────────────────────────────────────────────────────

token!(TypefaceBrand, Typeface);
token!(TypefacePlain, Typeface);
token!(
	/// Monospace family for code and pre elements.
	TypefaceMono,
	Typeface
);

// ── Weight ref tokens ─────────────────────────────────────────────────────────

token!(WeightRegular, FontWeight);
token!(WeightMedium,  FontWeight);
token!(WeightBold,    FontWeight);

// ── Font size ref tokens (MD3 type scale) ─────────────────────────────────────

token!(FontSizeDisplayLarge,   Length);
token!(FontSizeDisplayMedium,  Length);
token!(FontSizeDisplaySmall,   Length);
token!(FontSizeHeadlineLarge,  Length);
token!(FontSizeHeadlineMedium, Length);
token!(FontSizeHeadlineSmall,  Length);
token!(FontSizeTitleLarge,     Length);
token!(FontSizeTitleMedium,    Length);
token!(FontSizeTitleSmall,     Length);
token!(FontSizeBodyLarge,      Length);
token!(FontSizeBodyMedium,     Length);
token!(FontSizeBodySmall,      Length);
token!(FontSizeLabelLarge,     Length);
token!(FontSizeLabelMedium,    Length);
token!(FontSizeLabelSmall,     Length);

// ── Line height ref tokens (MD3 type scale) ───────────────────────────────────

token!(LineHeightDisplayLarge,   Length);
token!(LineHeightDisplayMedium,  Length);
token!(LineHeightDisplaySmall,   Length);
token!(LineHeightHeadlineLarge,  Length);
token!(LineHeightHeadlineMedium, Length);
token!(LineHeightHeadlineSmall,  Length);
token!(LineHeightTitleLarge,     Length);
token!(LineHeightTitleMedium,    Length);
token!(LineHeightTitleSmall,     Length);
token!(LineHeightBodyLarge,      Length);
token!(LineHeightBodyMedium,     Length);
token!(LineHeightBodySmall,      Length);
token!(LineHeightLabelLarge,     Length);
token!(LineHeightLabelMedium,    Length);
token!(LineHeightLabelSmall,     Length);

// ── Sys tokens: composite typography scales ───────────────────────────────────

token!(DisplayLarge,   Typography);
token!(DisplayMedium,  Typography);
token!(DisplaySmall,   Typography);
token!(HeadlineLarge,  Typography);
token!(HeadlineMedium, Typography);
token!(HeadlineSmall,  Typography);
token!(TitleLarge,     Typography);
token!(TitleMedium,    Typography);
token!(TitleSmall,     Typography);
token!(BodyLarge,      Typography);
token!(BodyMedium,     Typography);
token!(BodySmall,      Typography);
token!(LabelLarge,     Typography);
token!(LabelMedium,    Typography);
token!(LabelSmall,     Typography);

/// Returns a [`Rule`] with all MD3 typography default values.
///
/// Includes ref tokens (typefaces, weights, font sizes, line heights)
/// and sys tokens (15 composite typescale entries).
pub fn default_typography() -> Rule {
	Rule::root()
		// ── Typeface ref tokens ───────────────────────────────────────────────
		.with_value::<TypefacePlain>(Typeface::new(["Google Sans", "Product Sans", "Inter", "Work Sans", "system-ui", "sans-serif"])).unwrap()
		.with_value::<TypefaceBrand>(Typeface::new(["Roboto", "system-ui", "-apple-system", "BlinkMacSystemFont", "Segoe UI", "sans-serif"])).unwrap()
		.with_value::<TypefaceMono>(Typeface::new(["Roboto Mono", "'Courier New'", "monospace"])).unwrap()
		// ── Weight ref tokens ─────────────────────────────────────────────────
		.with_value::<WeightRegular>(FontWeight::Absolute(400)).unwrap()
		.with_value::<WeightMedium>(FontWeight::Absolute(500)).unwrap()
		.with_value::<WeightBold>(FontWeight::Absolute(700)).unwrap()
		// ── Font size ref tokens (MD3 sp → rem at 16 px base) ─────────────────
		.with_value::<FontSizeDisplayLarge>(Length::rem(3.5625)).unwrap()
		.with_value::<FontSizeDisplayMedium>(Length::rem(2.8125)).unwrap()
		.with_value::<FontSizeDisplaySmall>(Length::rem(2.25)).unwrap()
		.with_value::<FontSizeHeadlineLarge>(Length::rem(2.0)).unwrap()
		.with_value::<FontSizeHeadlineMedium>(Length::rem(1.75)).unwrap()
		.with_value::<FontSizeHeadlineSmall>(Length::rem(1.5)).unwrap()
		.with_value::<FontSizeTitleLarge>(Length::rem(1.375)).unwrap()
		.with_value::<FontSizeTitleMedium>(Length::rem(1.0)).unwrap()
		.with_value::<FontSizeTitleSmall>(Length::rem(0.875)).unwrap()
		.with_value::<FontSizeBodyLarge>(Length::rem(1.0)).unwrap()
		.with_value::<FontSizeBodyMedium>(Length::rem(0.875)).unwrap()
		.with_value::<FontSizeBodySmall>(Length::rem(0.75)).unwrap()
		.with_value::<FontSizeLabelLarge>(Length::rem(0.875)).unwrap()
		.with_value::<FontSizeLabelMedium>(Length::rem(0.75)).unwrap()
		.with_value::<FontSizeLabelSmall>(Length::rem(0.6875)).unwrap()
		// ── Line height ref tokens (MD3 sp → rem at 16 px base) ───────────────
		.with_value::<LineHeightDisplayLarge>(Length::rem(4.0)).unwrap()
		.with_value::<LineHeightDisplayMedium>(Length::rem(3.25)).unwrap()
		.with_value::<LineHeightDisplaySmall>(Length::rem(2.75)).unwrap()
		.with_value::<LineHeightHeadlineLarge>(Length::rem(2.5)).unwrap()
		.with_value::<LineHeightHeadlineMedium>(Length::rem(2.25)).unwrap()
		.with_value::<LineHeightHeadlineSmall>(Length::rem(2.0)).unwrap()
		.with_value::<LineHeightTitleLarge>(Length::rem(1.75)).unwrap()
		.with_value::<LineHeightTitleMedium>(Length::rem(1.5)).unwrap()
		.with_value::<LineHeightTitleSmall>(Length::rem(1.25)).unwrap()
		.with_value::<LineHeightBodyLarge>(Length::rem(1.5)).unwrap()
		.with_value::<LineHeightBodyMedium>(Length::rem(1.25)).unwrap()
		.with_value::<LineHeightBodySmall>(Length::rem(1.0)).unwrap()
		.with_value::<LineHeightLabelLarge>(Length::rem(1.25)).unwrap()
		.with_value::<LineHeightLabelMedium>(Length::rem(1.0)).unwrap()
		.with_value::<LineHeightLabelSmall>(Length::rem(1.0)).unwrap()
		// ── Composite typography sys tokens ───────────────────────────────────
		.with_value::<DisplayLarge>(Typography   { typeface: TypefaceBrand::token(), weight: WeightRegular::token(), size: Length::rem(3.5625), line_height: None, letter_spacing: None }).unwrap()
		.with_value::<DisplayMedium>(Typography  { typeface: TypefaceBrand::token(), weight: WeightRegular::token(), size: Length::rem(2.8125), line_height: None, letter_spacing: None }).unwrap()
		.with_value::<DisplaySmall>(Typography   { typeface: TypefaceBrand::token(), weight: WeightRegular::token(), size: Length::rem(2.25),   line_height: None, letter_spacing: None }).unwrap()
		.with_value::<HeadlineLarge>(Typography  { typeface: TypefacePlain::token(), weight: WeightRegular::token(), size: Length::rem(2.0),    line_height: None, letter_spacing: None }).unwrap()
		.with_value::<HeadlineMedium>(Typography { typeface: TypefacePlain::token(), weight: WeightRegular::token(), size: Length::rem(1.75),   line_height: None, letter_spacing: None }).unwrap()
		.with_value::<HeadlineSmall>(Typography  { typeface: TypefacePlain::token(), weight: WeightRegular::token(), size: Length::rem(1.5),    line_height: None, letter_spacing: None }).unwrap()
		.with_value::<TitleLarge>(Typography     { typeface: TypefacePlain::token(), weight: WeightRegular::token(), size: Length::rem(1.375),  line_height: None, letter_spacing: None }).unwrap()
		.with_value::<TitleMedium>(Typography    { typeface: TypefacePlain::token(), weight: WeightMedium::token(),  size: Length::rem(1.0),    line_height: None, letter_spacing: None }).unwrap()
		.with_value::<TitleSmall>(Typography     { typeface: TypefacePlain::token(), weight: WeightMedium::token(),  size: Length::rem(0.875),  line_height: None, letter_spacing: None }).unwrap()
		.with_value::<BodyLarge>(Typography      { typeface: TypefacePlain::token(), weight: WeightRegular::token(), size: Length::rem(1.0),    line_height: None, letter_spacing: None }).unwrap()
		.with_value::<BodyMedium>(Typography     { typeface: TypefacePlain::token(), weight: WeightRegular::token(), size: Length::rem(0.875),  line_height: None, letter_spacing: None }).unwrap()
		.with_value::<BodySmall>(Typography      { typeface: TypefacePlain::token(), weight: WeightRegular::token(), size: Length::rem(0.75),   line_height: None, letter_spacing: None }).unwrap()
		.with_value::<LabelLarge>(Typography     { typeface: TypefacePlain::token(), weight: WeightMedium::token(),  size: Length::rem(0.875),  line_height: None, letter_spacing: None }).unwrap()
		.with_value::<LabelMedium>(Typography    { typeface: TypefacePlain::token(), weight: WeightMedium::token(),  size: Length::rem(0.75),   line_height: None, letter_spacing: None }).unwrap()
		.with_value::<LabelSmall>(Typography     { typeface: TypefacePlain::token(), weight: WeightMedium::token(),  size: Length::rem(0.6875), line_height: None, letter_spacing: None }).unwrap()
}
