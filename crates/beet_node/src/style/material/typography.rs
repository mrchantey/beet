#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::*;


// ── Typeface ref tokens ───────────────────────────────────────────────────────

css_variable!(TypefaceBrand, Typeface);
css_variable!(TypefacePlain, Typeface);
css_variable!(
	/// Monospace family for code and pre elements.
	TypefaceMono,
	Typeface
);

// ── Weight ref tokens ─────────────────────────────────────────────────────────

css_variable!(WeightRegular, FontWeight);
css_variable!(WeightMedium,  FontWeight);
css_variable!(WeightBold,    FontWeight);

// ── Font size ref tokens (MD3 type scale) ─────────────────────────────────────

css_variable!(FontSizeDisplayLarge,   Length);
css_variable!(FontSizeDisplayMedium,  Length);
css_variable!(FontSizeDisplaySmall,   Length);
css_variable!(FontSizeHeadlineLarge,  Length);
css_variable!(FontSizeHeadlineMedium, Length);
css_variable!(FontSizeHeadlineSmall,  Length);
css_variable!(FontSizeTitleLarge,     Length);
css_variable!(FontSizeTitleMedium,    Length);
css_variable!(FontSizeTitleSmall,     Length);
css_variable!(FontSizeBodyLarge,      Length);
css_variable!(FontSizeBodyMedium,     Length);
css_variable!(FontSizeBodySmall,      Length);
css_variable!(FontSizeLabelLarge,     Length);
css_variable!(FontSizeLabelMedium,    Length);
css_variable!(FontSizeLabelSmall,     Length);

// ── Line height ref tokens (MD3 type scale) ───────────────────────────────────

css_variable!(LineHeightDisplayLarge,   Length);
css_variable!(LineHeightDisplayMedium,  Length);
css_variable!(LineHeightDisplaySmall,   Length);
css_variable!(LineHeightHeadlineLarge,  Length);
css_variable!(LineHeightHeadlineMedium, Length);
css_variable!(LineHeightHeadlineSmall,  Length);
css_variable!(LineHeightTitleLarge,     Length);
css_variable!(LineHeightTitleMedium,    Length);
css_variable!(LineHeightTitleSmall,     Length);
css_variable!(LineHeightBodyLarge,      Length);
css_variable!(LineHeightBodyMedium,     Length);
css_variable!(LineHeightBodySmall,      Length);
css_variable!(LineHeightLabelLarge,     Length);
css_variable!(LineHeightLabelMedium,    Length);
css_variable!(LineHeightLabelSmall,     Length);

// ── Sys tokens: composite typography scales ───────────────────────────────────

css_variable!(DisplayLarge,   Typography);
css_variable!(DisplayMedium,  Typography);
css_variable!(DisplaySmall,   Typography);
css_variable!(HeadlineLarge,  Typography);
css_variable!(HeadlineMedium, Typography);
css_variable!(HeadlineSmall,  Typography);
css_variable!(TitleLarge,     Typography);
css_variable!(TitleMedium,    Typography);
css_variable!(TitleSmall,     Typography);
css_variable!(BodyLarge,      Typography);
css_variable!(BodyMedium,     Typography);
css_variable!(BodySmall,      Typography);
css_variable!(LabelLarge,     Typography);
css_variable!(LabelMedium,    Typography);
css_variable!(LabelSmall,     Typography);


pub fn token_map() -> CssTokenMap {
	CssTokenMap::default()
		.insert(TypefaceBrand)
		.insert(TypefacePlain)
		.insert(TypefaceMono)
		.insert(WeightRegular)
		.insert(WeightMedium)
		.insert(WeightBold)
		.insert(FontSizeDisplayLarge)
		.insert(FontSizeDisplayMedium)
		.insert(FontSizeDisplaySmall)
		.insert(FontSizeHeadlineLarge)
		.insert(FontSizeHeadlineMedium)
		.insert(FontSizeHeadlineSmall)
		.insert(FontSizeTitleLarge)
		.insert(FontSizeTitleMedium)
		.insert(FontSizeTitleSmall)
		.insert(FontSizeBodyLarge)
		.insert(FontSizeBodyMedium)
		.insert(FontSizeBodySmall)
		.insert(FontSizeLabelLarge)
		.insert(FontSizeLabelMedium)
		.insert(FontSizeLabelSmall)
		.insert(LineHeightDisplayLarge)
		.insert(LineHeightDisplayMedium)
		.insert(LineHeightDisplaySmall)
		.insert(LineHeightHeadlineLarge)
		.insert(LineHeightHeadlineMedium)
		.insert(LineHeightHeadlineSmall)
		.insert(LineHeightTitleLarge)
		.insert(LineHeightTitleMedium)
		.insert(LineHeightTitleSmall)
		.insert(LineHeightBodyLarge)
		.insert(LineHeightBodyMedium)
		.insert(LineHeightBodySmall)
		.insert(LineHeightLabelLarge)
		.insert(LineHeightLabelMedium)
		.insert(LineHeightLabelSmall)
}



/// Returns a [`Rule`] with all MD3 typography default values.
///
/// Includes ref tokens (typefaces, weights, font sizes, line heights)
/// and sys tokens (15 composite typescale entries).
pub fn default_typography() -> Rule {
	Rule::new()
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
