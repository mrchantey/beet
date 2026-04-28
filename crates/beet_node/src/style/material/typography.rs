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
		.with_value::<FontSizeDisplayLarge>(Length::Rem(3.5625)).unwrap()
		.with_value::<FontSizeDisplayMedium>(Length::Rem(2.8125)).unwrap()
		.with_value::<FontSizeDisplaySmall>(Length::Rem(2.25)).unwrap()
		.with_value::<FontSizeHeadlineLarge>(Length::Rem(2.0)).unwrap()
		.with_value::<FontSizeHeadlineMedium>(Length::Rem(1.75)).unwrap()
		.with_value::<FontSizeHeadlineSmall>(Length::Rem(1.5)).unwrap()
		.with_value::<FontSizeTitleLarge>(Length::Rem(1.375)).unwrap()
		.with_value::<FontSizeTitleMedium>(Length::Rem(1.0)).unwrap()
		.with_value::<FontSizeTitleSmall>(Length::Rem(0.875)).unwrap()
		.with_value::<FontSizeBodyLarge>(Length::Rem(1.0)).unwrap()
		.with_value::<FontSizeBodyMedium>(Length::Rem(0.875)).unwrap()
		.with_value::<FontSizeBodySmall>(Length::Rem(0.75)).unwrap()
		.with_value::<FontSizeLabelLarge>(Length::Rem(0.875)).unwrap()
		.with_value::<FontSizeLabelMedium>(Length::Rem(0.75)).unwrap()
		.with_value::<FontSizeLabelSmall>(Length::Rem(0.6875)).unwrap()
		// ── Line height ref tokens (MD3 sp → rem at 16 px base) ───────────────
		.with_value::<LineHeightDisplayLarge>(Length::Rem(4.0)).unwrap()
		.with_value::<LineHeightDisplayMedium>(Length::Rem(3.25)).unwrap()
		.with_value::<LineHeightDisplaySmall>(Length::Rem(2.75)).unwrap()
		.with_value::<LineHeightHeadlineLarge>(Length::Rem(2.5)).unwrap()
		.with_value::<LineHeightHeadlineMedium>(Length::Rem(2.25)).unwrap()
		.with_value::<LineHeightHeadlineSmall>(Length::Rem(2.0)).unwrap()
		.with_value::<LineHeightTitleLarge>(Length::Rem(1.75)).unwrap()
		.with_value::<LineHeightTitleMedium>(Length::Rem(1.5)).unwrap()
		.with_value::<LineHeightTitleSmall>(Length::Rem(1.25)).unwrap()
		.with_value::<LineHeightBodyLarge>(Length::Rem(1.5)).unwrap()
		.with_value::<LineHeightBodyMedium>(Length::Rem(1.25)).unwrap()
		.with_value::<LineHeightBodySmall>(Length::Rem(1.0)).unwrap()
		.with_value::<LineHeightLabelLarge>(Length::Rem(1.25)).unwrap()
		.with_value::<LineHeightLabelMedium>(Length::Rem(1.0)).unwrap()
		.with_value::<LineHeightLabelSmall>(Length::Rem(1.0)).unwrap()
		// ── Letter spacing ref tokens (MD3 tracking) ──────────────────────────
		.with_value::<LetterSpacingDisplayLarge>(Length::Rem(-0.015625)).unwrap()
		.with_value::<LetterSpacingDisplayMedium>(Length::Rem(0.0)).unwrap()
		.with_value::<LetterSpacingDisplaySmall>(Length::Rem(0.0)).unwrap()
		.with_value::<LetterSpacingHeadlineLarge>(Length::Rem(0.0)).unwrap()
		.with_value::<LetterSpacingHeadlineMedium>(Length::Rem(0.0)).unwrap()
		.with_value::<LetterSpacingHeadlineSmall>(Length::Rem(0.0)).unwrap()
		.with_value::<LetterSpacingTitleLarge>(Length::Rem(0.0)).unwrap()
		.with_value::<LetterSpacingTitleMedium>(Length::Rem(0.009375)).unwrap()
		.with_value::<LetterSpacingTitleSmall>(Length::Rem(0.00625)).unwrap()
		.with_value::<LetterSpacingBodyLarge>(Length::Rem(0.03125)).unwrap()
		.with_value::<LetterSpacingBodyMedium>(Length::Rem(0.015625)).unwrap()
		.with_value::<LetterSpacingBodySmall>(Length::Rem(0.025)).unwrap()
		.with_value::<LetterSpacingLabelLarge>(Length::Rem(0.00625)).unwrap()
		.with_value::<LetterSpacingLabelMedium>(Length::Rem(0.03125)).unwrap()
		.with_value::<LetterSpacingLabelSmall>(Length::Rem(0.03125)).unwrap()
		// ── Composite typography sys tokens ───────────────────────────────────
		.with_value::<DisplayLarge>(Typography   { typeface: TypefaceBrand.into(), weight: WeightRegular.into(), size: FontSizeDisplayLarge.into(),   line_height: LineHeightDisplayLarge.into(),   letter_spacing: LetterSpacingDisplayLarge.into() }).unwrap()
		.with_value::<DisplayMedium>(Typography  { typeface: TypefaceBrand.into(), weight: WeightRegular.into(), size: FontSizeDisplayMedium.into(),  line_height: LineHeightDisplayMedium.into(),  letter_spacing: LetterSpacingDisplayMedium.into() }).unwrap()
		.with_value::<DisplaySmall>(Typography   { typeface: TypefaceBrand.into(), weight: WeightRegular.into(), size: FontSizeDisplaySmall.into(),   line_height: LineHeightDisplaySmall.into(),   letter_spacing: LetterSpacingDisplaySmall.into() }).unwrap()
		.with_value::<HeadlineLarge>(Typography  { typeface: TypefacePlain.into(), weight: WeightRegular.into(), size: FontSizeHeadlineLarge.into(),  line_height: LineHeightHeadlineLarge.into(),  letter_spacing: LetterSpacingHeadlineLarge.into() }).unwrap()
		.with_value::<HeadlineMedium>(Typography { typeface: TypefacePlain.into(), weight: WeightRegular.into(), size: FontSizeHeadlineMedium.into(), line_height: LineHeightHeadlineMedium.into(), letter_spacing: LetterSpacingHeadlineMedium.into() }).unwrap()
		.with_value::<HeadlineSmall>(Typography  { typeface: TypefacePlain.into(), weight: WeightRegular.into(), size: FontSizeHeadlineSmall.into(),  line_height: LineHeightHeadlineSmall.into(),  letter_spacing: LetterSpacingHeadlineSmall.into() }).unwrap()
		.with_value::<TitleLarge>(Typography     { typeface: TypefacePlain.into(), weight: WeightRegular.into(), size: FontSizeTitleLarge.into(),     line_height: LineHeightTitleLarge.into(),     letter_spacing: LetterSpacingTitleLarge.into() }).unwrap()
		.with_value::<TitleMedium>(Typography    { typeface: TypefacePlain.into(), weight: WeightMedium.into(),  size: FontSizeTitleMedium.into(),    line_height: LineHeightTitleMedium.into(),    letter_spacing: LetterSpacingTitleMedium.into() }).unwrap()
		.with_value::<TitleSmall>(Typography     { typeface: TypefacePlain.into(), weight: WeightMedium.into(),  size: FontSizeTitleSmall.into(),     line_height: LineHeightTitleSmall.into(),     letter_spacing: LetterSpacingTitleSmall.into() }).unwrap()
		.with_value::<BodyLarge>(Typography      { typeface: TypefacePlain.into(), weight: WeightRegular.into(), size: FontSizeBodyLarge.into(),      line_height: LineHeightBodyLarge.into(),      letter_spacing: LetterSpacingBodyLarge.into() }).unwrap()
		.with_value::<BodyMedium>(Typography     { typeface: TypefacePlain.into(), weight: WeightRegular.into(), size: FontSizeBodyMedium.into(),     line_height: LineHeightBodyMedium.into(),     letter_spacing: LetterSpacingBodyMedium.into() }).unwrap()
		.with_value::<BodySmall>(Typography      { typeface: TypefacePlain.into(), weight: WeightRegular.into(), size: FontSizeBodySmall.into(),      line_height: LineHeightBodySmall.into(),      letter_spacing: LetterSpacingBodySmall.into() }).unwrap()
		.with_value::<LabelLarge>(Typography     { typeface: TypefacePlain.into(), weight: WeightMedium.into(),  size: FontSizeLabelLarge.into(),     line_height: LineHeightLabelLarge.into(),     letter_spacing: LetterSpacingLabelLarge.into() }).unwrap()
		.with_value::<LabelMedium>(Typography    { typeface: TypefacePlain.into(), weight: WeightMedium.into(),  size: FontSizeLabelMedium.into(),    line_height: LineHeightLabelMedium.into(),    letter_spacing: LetterSpacingLabelMedium.into() }).unwrap()
		.with_value::<LabelSmall>(Typography     { typeface: TypefacePlain.into(), weight: WeightMedium.into(),  size: FontSizeLabelSmall.into(),     line_height: LineHeightLabelSmall.into(),     letter_spacing: LetterSpacingLabelSmall.into() }).unwrap()
}
