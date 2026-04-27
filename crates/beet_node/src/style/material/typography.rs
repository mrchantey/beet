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

// ── Letter spacing ref tokens (MD3 tracking values) ───────────────────────────

css_variable!(LetterSpacingDisplayLarge,   Length);
css_variable!(LetterSpacingDisplayMedium,  Length);
css_variable!(LetterSpacingDisplaySmall,   Length);
css_variable!(LetterSpacingHeadlineLarge,  Length);
css_variable!(LetterSpacingHeadlineMedium, Length);
css_variable!(LetterSpacingHeadlineSmall,  Length);
css_variable!(LetterSpacingTitleLarge,     Length);
css_variable!(LetterSpacingTitleMedium,    Length);
css_variable!(LetterSpacingTitleSmall,     Length);
css_variable!(LetterSpacingBodyLarge,      Length);
css_variable!(LetterSpacingBodyMedium,     Length);
css_variable!(LetterSpacingBodySmall,      Length);
css_variable!(LetterSpacingLabelLarge,     Length);
css_variable!(LetterSpacingLabelMedium,    Length);
css_variable!(LetterSpacingLabelSmall,     Length);

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
		.insert(LetterSpacingDisplayLarge)
		.insert(LetterSpacingDisplayMedium)
		.insert(LetterSpacingDisplaySmall)
		.insert(LetterSpacingHeadlineLarge)
		.insert(LetterSpacingHeadlineMedium)
		.insert(LetterSpacingHeadlineSmall)
		.insert(LetterSpacingTitleLarge)
		.insert(LetterSpacingTitleMedium)
		.insert(LetterSpacingTitleSmall)
		.insert(LetterSpacingBodyLarge)
		.insert(LetterSpacingBodyMedium)
		.insert(LetterSpacingBodySmall)
		.insert(LetterSpacingLabelLarge)
		.insert(LetterSpacingLabelMedium)
		.insert(LetterSpacingLabelSmall)
		.insert(TypographyProps)
		.insert(DisplayLarge)
		.insert(DisplayMedium)
		.insert(DisplaySmall)
		.insert(HeadlineLarge)
		.insert(HeadlineMedium)
		.insert(HeadlineSmall)
		.insert(TitleLarge)
		.insert(TitleMedium)
		.insert(TitleSmall)
		.insert(BodyLarge)
		.insert(BodyMedium)
		.insert(BodySmall)
		.insert(LabelLarge)
		.insert(LabelMedium)
		.insert(LabelSmall)
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
		// ── Letter spacing ref tokens (MD3 tracking) ──────────────────────────
		.with_value::<LetterSpacingDisplayLarge>(Length::rem(-0.015625)).unwrap()
		.with_value::<LetterSpacingDisplayMedium>(Length::rem(0.0)).unwrap()
		.with_value::<LetterSpacingDisplaySmall>(Length::rem(0.0)).unwrap()
		.with_value::<LetterSpacingHeadlineLarge>(Length::rem(0.0)).unwrap()
		.with_value::<LetterSpacingHeadlineMedium>(Length::rem(0.0)).unwrap()
		.with_value::<LetterSpacingHeadlineSmall>(Length::rem(0.0)).unwrap()
		.with_value::<LetterSpacingTitleLarge>(Length::rem(0.0)).unwrap()
		.with_value::<LetterSpacingTitleMedium>(Length::rem(0.009375)).unwrap()
		.with_value::<LetterSpacingTitleSmall>(Length::rem(0.00625)).unwrap()
		.with_value::<LetterSpacingBodyLarge>(Length::rem(0.03125)).unwrap()
		.with_value::<LetterSpacingBodyMedium>(Length::rem(0.015625)).unwrap()
		.with_value::<LetterSpacingBodySmall>(Length::rem(0.025)).unwrap()
		.with_value::<LetterSpacingLabelLarge>(Length::rem(0.00625)).unwrap()
		.with_value::<LetterSpacingLabelMedium>(Length::rem(0.03125)).unwrap()
		.with_value::<LetterSpacingLabelSmall>(Length::rem(0.03125)).unwrap()
		// ── Composite typography sys tokens ───────────────────────────────────
		.with_value::<DisplayLarge>(Typography   { typeface: TypefaceBrand::token(), weight: WeightRegular::token(), size: FontSizeDisplayLarge::token(),   line_height: LineHeightDisplayLarge::token(),   letter_spacing: LetterSpacingDisplayLarge::token() }).unwrap()
		.with_value::<DisplayMedium>(Typography  { typeface: TypefaceBrand::token(), weight: WeightRegular::token(), size: FontSizeDisplayMedium::token(),  line_height: LineHeightDisplayMedium::token(),  letter_spacing: LetterSpacingDisplayMedium::token() }).unwrap()
		.with_value::<DisplaySmall>(Typography   { typeface: TypefaceBrand::token(), weight: WeightRegular::token(), size: FontSizeDisplaySmall::token(),   line_height: LineHeightDisplaySmall::token(),   letter_spacing: LetterSpacingDisplaySmall::token() }).unwrap()
		.with_value::<HeadlineLarge>(Typography  { typeface: TypefacePlain::token(), weight: WeightRegular::token(), size: FontSizeHeadlineLarge::token(),  line_height: LineHeightHeadlineLarge::token(),  letter_spacing: LetterSpacingHeadlineLarge::token() }).unwrap()
		.with_value::<HeadlineMedium>(Typography { typeface: TypefacePlain::token(), weight: WeightRegular::token(), size: FontSizeHeadlineMedium::token(), line_height: LineHeightHeadlineMedium::token(), letter_spacing: LetterSpacingHeadlineMedium::token() }).unwrap()
		.with_value::<HeadlineSmall>(Typography  { typeface: TypefacePlain::token(), weight: WeightRegular::token(), size: FontSizeHeadlineSmall::token(),  line_height: LineHeightHeadlineSmall::token(),  letter_spacing: LetterSpacingHeadlineSmall::token() }).unwrap()
		.with_value::<TitleLarge>(Typography     { typeface: TypefacePlain::token(), weight: WeightRegular::token(), size: FontSizeTitleLarge::token(),     line_height: LineHeightTitleLarge::token(),     letter_spacing: LetterSpacingTitleLarge::token() }).unwrap()
		.with_value::<TitleMedium>(Typography    { typeface: TypefacePlain::token(), weight: WeightMedium::token(),  size: FontSizeTitleMedium::token(),    line_height: LineHeightTitleMedium::token(),    letter_spacing: LetterSpacingTitleMedium::token() }).unwrap()
		.with_value::<TitleSmall>(Typography     { typeface: TypefacePlain::token(), weight: WeightMedium::token(),  size: FontSizeTitleSmall::token(),     line_height: LineHeightTitleSmall::token(),     letter_spacing: LetterSpacingTitleSmall::token() }).unwrap()
		.with_value::<BodyLarge>(Typography      { typeface: TypefacePlain::token(), weight: WeightRegular::token(), size: FontSizeBodyLarge::token(),      line_height: LineHeightBodyLarge::token(),      letter_spacing: LetterSpacingBodyLarge::token() }).unwrap()
		.with_value::<BodyMedium>(Typography     { typeface: TypefacePlain::token(), weight: WeightRegular::token(), size: FontSizeBodyMedium::token(),     line_height: LineHeightBodyMedium::token(),     letter_spacing: LetterSpacingBodyMedium::token() }).unwrap()
		.with_value::<BodySmall>(Typography      { typeface: TypefacePlain::token(), weight: WeightRegular::token(), size: FontSizeBodySmall::token(),      line_height: LineHeightBodySmall::token(),      letter_spacing: LetterSpacingBodySmall::token() }).unwrap()
		.with_value::<LabelLarge>(Typography     { typeface: TypefacePlain::token(), weight: WeightMedium::token(),  size: FontSizeLabelLarge::token(),     line_height: LineHeightLabelLarge::token(),     letter_spacing: LetterSpacingLabelLarge::token() }).unwrap()
		.with_value::<LabelMedium>(Typography    { typeface: TypefacePlain::token(), weight: WeightMedium::token(),  size: FontSizeLabelMedium::token(),    line_height: LineHeightLabelMedium::token(),    letter_spacing: LetterSpacingLabelMedium::token() }).unwrap()
		.with_value::<LabelSmall>(Typography     { typeface: TypefacePlain::token(), weight: WeightMedium::token(),  size: FontSizeLabelSmall::token(),     line_height: LineHeightLabelSmall::token(),     letter_spacing: LetterSpacingLabelSmall::token() }).unwrap()
}
