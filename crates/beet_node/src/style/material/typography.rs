#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::*;


// ── Typeface ref tokens ───────────────────────────────────────────────────────

token2!(TypefaceBrand, Typeface);
token2!(TypefacePlain, Typeface);
token2!(
	/// Monospace family for code and pre elements.
	TypefaceMono,
	Typeface
);

// ── Weight ref tokens ─────────────────────────────────────────────────────────

token2!(WeightRegular, FontWeight);
token2!(WeightMedium,  FontWeight);
token2!(WeightBold,    FontWeight);

// ── Font size ref tokens (MD3 type scale) ─────────────────────────────────────

token2!(FontSizeDisplayLarge,   Length);
token2!(FontSizeDisplayMedium,  Length);
token2!(FontSizeDisplaySmall,   Length);
token2!(FontSizeHeadlineLarge,  Length);
token2!(FontSizeHeadlineMedium, Length);
token2!(FontSizeHeadlineSmall,  Length);
token2!(FontSizeTitleLarge,     Length);
token2!(FontSizeTitleMedium,    Length);
token2!(FontSizeTitleSmall,     Length);
token2!(FontSizeBodyLarge,      Length);
token2!(FontSizeBodyMedium,     Length);
token2!(FontSizeBodySmall,      Length);
token2!(FontSizeLabelLarge,     Length);
token2!(FontSizeLabelMedium,    Length);
token2!(FontSizeLabelSmall,     Length);

// ── Line height ref tokens (MD3 type scale) ───────────────────────────────────

token2!(LineHeightDisplayLarge,   Length);
token2!(LineHeightDisplayMedium,  Length);
token2!(LineHeightDisplaySmall,   Length);
token2!(LineHeightHeadlineLarge,  Length);
token2!(LineHeightHeadlineMedium, Length);
token2!(LineHeightHeadlineSmall,  Length);
token2!(LineHeightTitleLarge,     Length);
token2!(LineHeightTitleMedium,    Length);
token2!(LineHeightTitleSmall,     Length);
token2!(LineHeightBodyLarge,      Length);
token2!(LineHeightBodyMedium,     Length);
token2!(LineHeightBodySmall,      Length);
token2!(LineHeightLabelLarge,     Length);
token2!(LineHeightLabelMedium,    Length);
token2!(LineHeightLabelSmall,     Length);

// ── Sys tokens: composite typography scales ───────────────────────────────────

token2!(DisplayLarge,   Typography);
token2!(DisplayMedium,  Typography);
token2!(DisplaySmall,   Typography);
token2!(HeadlineLarge,  Typography);
token2!(HeadlineMedium, Typography);
token2!(HeadlineSmall,  Typography);
token2!(TitleLarge,     Typography);
token2!(TitleMedium,    Typography);
token2!(TitleSmall,     Typography);
token2!(BodyLarge,      Typography);
token2!(BodyMedium,     Typography);
token2!(BodySmall,      Typography);
token2!(LabelLarge,     Typography);
token2!(LabelMedium,    Typography);
token2!(LabelSmall,     Typography);

/// Returns a [`Selector`] with all MD3 typography default values.
///
/// Includes ref tokens (typefaces, weights, font sizes, line heights)
/// and sys tokens (15 composite typescale entries).
pub fn default_typography() -> Selector {
	Selector::new()
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
		.with_value::<DisplayLarge>(Typography   { typeface: FieldRef::of::<TypefaceBrand>(), weight: FieldRef::of::<WeightRegular>(), size: Length::rem(3.5625), line_height: None, letter_spacing: None }).unwrap()
		.with_value::<DisplayMedium>(Typography  { typeface: FieldRef::of::<TypefaceBrand>(), weight: FieldRef::of::<WeightRegular>(), size: Length::rem(2.8125), line_height: None, letter_spacing: None }).unwrap()
		.with_value::<DisplaySmall>(Typography   { typeface: FieldRef::of::<TypefaceBrand>(), weight: FieldRef::of::<WeightRegular>(), size: Length::rem(2.25),   line_height: None, letter_spacing: None }).unwrap()
		.with_value::<HeadlineLarge>(Typography  { typeface: FieldRef::of::<TypefacePlain>(), weight: FieldRef::of::<WeightRegular>(), size: Length::rem(2.0),    line_height: None, letter_spacing: None }).unwrap()
		.with_value::<HeadlineMedium>(Typography { typeface: FieldRef::of::<TypefacePlain>(), weight: FieldRef::of::<WeightRegular>(), size: Length::rem(1.75),   line_height: None, letter_spacing: None }).unwrap()
		.with_value::<HeadlineSmall>(Typography  { typeface: FieldRef::of::<TypefacePlain>(), weight: FieldRef::of::<WeightRegular>(), size: Length::rem(1.5),    line_height: None, letter_spacing: None }).unwrap()
		.with_value::<TitleLarge>(Typography     { typeface: FieldRef::of::<TypefacePlain>(), weight: FieldRef::of::<WeightRegular>(), size: Length::rem(1.375),  line_height: None, letter_spacing: None }).unwrap()
		.with_value::<TitleMedium>(Typography    { typeface: FieldRef::of::<TypefacePlain>(), weight: FieldRef::of::<WeightMedium>(),  size: Length::rem(1.0),    line_height: None, letter_spacing: None }).unwrap()
		.with_value::<TitleSmall>(Typography     { typeface: FieldRef::of::<TypefacePlain>(), weight: FieldRef::of::<WeightMedium>(),  size: Length::rem(0.875),  line_height: None, letter_spacing: None }).unwrap()
		.with_value::<BodyLarge>(Typography      { typeface: FieldRef::of::<TypefacePlain>(), weight: FieldRef::of::<WeightRegular>(), size: Length::rem(1.0),    line_height: None, letter_spacing: None }).unwrap()
		.with_value::<BodyMedium>(Typography     { typeface: FieldRef::of::<TypefacePlain>(), weight: FieldRef::of::<WeightRegular>(), size: Length::rem(0.875),  line_height: None, letter_spacing: None }).unwrap()
		.with_value::<BodySmall>(Typography      { typeface: FieldRef::of::<TypefacePlain>(), weight: FieldRef::of::<WeightRegular>(), size: Length::rem(0.75),   line_height: None, letter_spacing: None }).unwrap()
		.with_value::<LabelLarge>(Typography     { typeface: FieldRef::of::<TypefacePlain>(), weight: FieldRef::of::<WeightMedium>(),  size: Length::rem(0.875),  line_height: None, letter_spacing: None }).unwrap()
		.with_value::<LabelMedium>(Typography    { typeface: FieldRef::of::<TypefacePlain>(), weight: FieldRef::of::<WeightMedium>(),  size: Length::rem(0.75),   line_height: None, letter_spacing: None }).unwrap()
		.with_value::<LabelSmall>(Typography     { typeface: FieldRef::of::<TypefacePlain>(), weight: FieldRef::of::<WeightMedium>(),  size: Length::rem(0.6875), line_height: None, letter_spacing: None }).unwrap()
}
