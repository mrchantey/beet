//! Typography classes, prose element overrides and their Material Design 3 rules.
#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::*;
use crate::style::material::*;

// ── Typography scale ────────────────────────────────────────────────────────────
pub const TEXT_DISPLAY_LARGE: ClassName = ClassName::new_static("text-display-large");
pub const TEXT_DISPLAY_MEDIUM: ClassName = ClassName::new_static("text-display-medium");
pub const TEXT_DISPLAY_SMALL: ClassName = ClassName::new_static("text-display-small");
pub const TEXT_HEADLINE_LARGE: ClassName = ClassName::new_static("text-headline-large");
pub const TEXT_HEADLINE_MEDIUM: ClassName = ClassName::new_static("text-headline-medium");
pub const TEXT_HEADLINE_SMALL: ClassName = ClassName::new_static("text-headline-small");
pub const TEXT_TITLE_LARGE: ClassName = ClassName::new_static("text-title-large");
pub const TEXT_TITLE_MEDIUM: ClassName = ClassName::new_static("text-title-medium");
pub const TEXT_TITLE_SMALL: ClassName = ClassName::new_static("text-title-small");
pub const TEXT_BODY_LARGE: ClassName = ClassName::new_static("text-body-large");
pub const TEXT_BODY_MEDIUM: ClassName = ClassName::new_static("text-body-medium");
pub const TEXT_BODY_SMALL: ClassName = ClassName::new_static("text-body-small");
pub const TEXT_LABEL_LARGE: ClassName = ClassName::new_static("text-label-large");
pub const TEXT_LABEL_MEDIUM: ClassName = ClassName::new_static("text-label-medium");
pub const TEXT_LABEL_SMALL: ClassName = ClassName::new_static("text-label-small");

// ── Generic text utilities ──────────────────────────────────────────────────────
pub const TEXT_LEFT: ClassName = ClassName::new_static("text-left");
pub const TEXT_CENTER: ClassName = ClassName::new_static("text-center");
pub const TEXT_RIGHT: ClassName = ClassName::new_static("text-right");
pub const TEXT_XS: ClassName = ClassName::new_static("text-xs");
pub const TEXT_SM: ClassName = ClassName::new_static("text-sm");
pub const TEXT_BASE: ClassName = ClassName::new_static("text-base");
pub const TEXT_LG: ClassName = ClassName::new_static("text-lg");
pub const TEXT_XL: ClassName = ClassName::new_static("text-xl");
pub const TEXT_2XL: ClassName = ClassName::new_static("text-2xl");

// ── Typography scale rules ──────────────────────────────────────────────────────
//
// Each class sets the MD3 [`Typography`] composite (which serializes the whole
// type scale to CSS for the web) and, in addition, the longhand `font-size`. The
// charcell renderer scales glyphs by `font-size`, not the composite (a separate
// cascade token), so the size is set as a longhand too; it carries the class's
// specificity, resolving with normal precedence (an element's own size beats an
// inherited one, a class beats a tag).

/// Display large - largest hero text.
pub fn text_display_large() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_DISPLAY_LARGE))
		.with_token(TypographyProps,typography::DisplayLarge).unwrap()
		.with_token(common_props::FontSize,typography::FontSizeDisplayLarge).unwrap()
}

/// Display medium - medium hero text.
pub fn text_display_medium() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_DISPLAY_MEDIUM))
		.with_token(TypographyProps,typography::DisplayMedium).unwrap()
		.with_token(common_props::FontSize,typography::FontSizeDisplayMedium).unwrap()
}

/// Display small - small hero text.
pub fn text_display_small() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_DISPLAY_SMALL))
		.with_token(TypographyProps,typography::DisplaySmall).unwrap()
		.with_token(common_props::FontSize,typography::FontSizeDisplaySmall).unwrap()
}

/// Headline large - large section heading.
pub fn text_headline_large() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_HEADLINE_LARGE))
		.with_token(TypographyProps,typography::HeadlineLarge).unwrap()
		.with_token(common_props::FontSize,typography::FontSizeHeadlineLarge).unwrap()
}

/// Headline medium - medium section heading.
pub fn text_headline_medium() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_HEADLINE_MEDIUM))
		.with_token(TypographyProps,typography::HeadlineMedium).unwrap()
		.with_token(common_props::FontSize,typography::FontSizeHeadlineMedium).unwrap()
}

/// Headline small - small section heading.
pub fn text_headline_small() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_HEADLINE_SMALL))
		.with_token(TypographyProps,typography::HeadlineSmall).unwrap()
		.with_token(common_props::FontSize,typography::FontSizeHeadlineSmall).unwrap()
}

/// Title large - large title text.
pub fn text_title_large() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_TITLE_LARGE))
		.with_token(TypographyProps,typography::TitleLarge).unwrap()
		.with_token(common_props::FontSize,typography::FontSizeTitleLarge).unwrap()
}

/// Title medium - medium title text.
pub fn text_title_medium() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_TITLE_MEDIUM))
		.with_token(TypographyProps,typography::TitleMedium).unwrap()
		.with_token(common_props::FontSize,typography::FontSizeTitleMedium).unwrap()
}

/// Title small - small title text.
pub fn text_title_small() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_TITLE_SMALL))
		.with_token(TypographyProps,typography::TitleSmall).unwrap()
		.with_token(common_props::FontSize,typography::FontSizeTitleSmall).unwrap()
}

/// Body large - large body text.
pub fn text_body_large() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_BODY_LARGE))
		.with_token(TypographyProps,typography::BodyLarge).unwrap()
		.with_token(common_props::FontSize,typography::FontSizeBodyLarge).unwrap()
}

/// Body medium - medium body text (default).
pub fn text_body_medium() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_BODY_MEDIUM))
		.with_token(TypographyProps,typography::BodyMedium).unwrap()
		.with_token(common_props::FontSize,typography::FontSizeBodyMedium).unwrap()
}

/// Body small - small body text.
pub fn text_body_small() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_BODY_SMALL))
		.with_token(TypographyProps,typography::BodySmall).unwrap()
		.with_token(common_props::FontSize,typography::FontSizeBodySmall).unwrap()
}

/// Label large - large label text.
pub fn text_label_large() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_LABEL_LARGE))
		.with_token(TypographyProps,typography::LabelLarge).unwrap()
		.with_token(common_props::FontSize,typography::FontSizeLabelLarge).unwrap()
}

/// Label medium - medium label text.
pub fn text_label_medium() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_LABEL_MEDIUM))
		.with_token(TypographyProps,typography::LabelMedium).unwrap()
		.with_token(common_props::FontSize,typography::FontSizeLabelMedium).unwrap()
}

/// Label small - small label text.
pub fn text_label_small() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_LABEL_SMALL))
		.with_token(TypographyProps,typography::LabelSmall).unwrap()
		.with_token(common_props::FontSize,typography::FontSizeLabelSmall).unwrap()
}

// ── Prose element overrides ───────────────────────────────────────────────────

// Theme overrides for prose tags also styled by the user-agent
// [`default_element_rules`](crate::style::default_element_rules). Appended after
// them in `all_rules`, so the later (theme) rule wins the same-specificity tag
// cascade on both the terminal and the serialized stylesheet: links pick up
// `Primary`, code spans/blocks a `SurfaceContainerHighest` fill with `OnSurface`
// text.

/// Anchor text in the theme's primary color.
pub fn link_prose() -> Rule {
	Rule::new()
		.with_selector(Selector::tag("a"))
		.with_token(common_props::ForegroundColor,colors::Primary).unwrap()
}

/// The terminal's `<img>`/`<iframe>` link fallbacks in the same primary color,
/// so the alt/title placeholders read as themed links. Terminal-gated: on the
/// web these are a real image/frame, not links.
pub fn link_fallback_prose() -> Rule {
	Rule::new()
		.with_selector(Selector::AnyOf(vec![
			Selector::tag("img"),
			Selector::tag("iframe"),
		]))
		.with_media(MediaQuery::Terminal)
		.with_token(common_props::ForegroundColor,colors::Primary).unwrap()
}

/// Highlighted `<mark>` text - the secondary container fill matching the drag
/// selection (see the web `::selection` rule), so a highlight reads on-palette
/// on both targets rather than the browser's default yellow.
pub fn mark_prose() -> Rule {
	Rule::new()
		.with_selector(Selector::tag("mark"))
		.with_token(common_props::BackgroundColor,colors::Secondary).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnSecondary).unwrap()
}

/// Inline `<code>` - filled chip readable against the page surface, with a
/// faint rounded corner and a slim inset so the fill clears the glyphs. The
/// vertical inset never disturbs line height: on the web `<code>` is inline, so
/// top/bottom padding extends the chip background without growing the line box;
/// on the terminal the inset rounds to zero rows.
pub fn code_prose() -> Rule {
	Rule::new()
		.with_selector(Selector::tag("code"))
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
		.with_token(common_props::BackgroundColor,colors::SurfaceContainerHighest).unwrap()
		.with_token(ShapeProps,geometry::ShapeExtraSmall).unwrap()
		.with_value(common_props::Padding, Spacing {
			top: Length::Rem(0.1),
			bottom: Length::Rem(0.1),
			left: Length::Rem(0.3),
			right: Length::Rem(0.3),
		})
}

/// Block `<pre>` - filled code surface matching inline code, padded with a
/// rounded corner.
pub fn pre_prose() -> Rule {
	Rule::new()
		.with_selector(Selector::tag("pre"))
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
		.with_token(common_props::BackgroundColor,colors::SurfaceContainerHighest).unwrap()
		.with_token(ShapeProps,geometry::ShapeSmall).unwrap()
		.with_value(common_props::Padding, Spacing::all(Length::Rem(1.)))
}

/// Block `<blockquote>` - a flat tonal callout with an italic body and a primary
/// left rule, the look shared by web and terminal. A plain `surface-container-low`
/// fill (no elevation shadow, which would fight the flat surface) keeps it
/// reading as inset quoted text rather than a raised card.
pub fn blockquote_prose() -> Rule {
	Rule::new()
		.with_selector(Selector::tag("blockquote"))
		.with_token(common_props::BackgroundColor,colors::SurfaceContainer).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnSurfaceVariant).unwrap()
		.with_token(common_props::BorderColorProp,colors::Primary).unwrap()
		.with_token(common_props::BorderLeftWidth,geometry::OutlineWidthThick).unwrap()
		.with_token(ShapeProps,geometry::ShapeExtraSmall).unwrap()
		.with_value(common_props::Padding, Spacing::all(Length::Rem(1.)))
}

/// Terminal-only heading color - every heading level renders in the theme's
/// `Primary`, so headings read as the brand accent against the body text. Gated
/// behind [`MediaQuery::Terminal`] so the web and print stay plain.
pub fn terminal_headings() -> Rule {
	Rule::tags(&["h1", "h2", "h3", "h4", "h5", "h6"])
		.with_media(MediaQuery::Terminal)
		.with_token(common_props::ForegroundColor,colors::Primary).unwrap()
}

/// Prose heading sizes - maps `<h1>`..`<h6>` onto the MD3 type scale (headline
/// then title sizes) so headings step down in size as on the web reference,
/// rather than all rendering at the body size. Only `font-size`/`line-height`
/// are set. The terminal honours the `font-size` too, scaling headings to
/// fullwidth (`> 1em`) or the box-drawing block font (`> 2em`), both rendered
/// hardcoded-bold; see [`FontScale`](crate::render::FontScale).
pub fn heading_sizes() -> Vec<Rule> {
	vec![
		heading_size("h1", typography::FontSizeHeadlineLarge,  typography::LineHeightHeadlineLarge),
		heading_size("h2", typography::FontSizeHeadlineMedium, typography::LineHeightHeadlineMedium),
		heading_size("h3", typography::FontSizeHeadlineSmall,  typography::LineHeightHeadlineSmall),
		heading_size("h4", typography::FontSizeTitleLarge,     typography::LineHeightTitleLarge),
		heading_size("h5", typography::FontSizeTitleMedium,    typography::LineHeightTitleMedium),
		heading_size("h6", typography::FontSizeTitleSmall,     typography::LineHeightTitleSmall),
	]
}

/// One heading-level size rule, setting the font size and matching line height.
fn heading_size(tag: &str, size: impl Into<Token>, line_height: impl Into<Token>) -> Rule {
	Rule::tags(&[tag])
		.with_token(common_props::FontSize, size).unwrap()
		.with_token(common_props::LineHeight, line_height).unwrap()
}

// ── Generic text utility rules ──────────────────────────────────────────────────

/// A text-alignment utility rule for `class`.
pub fn text_align(class: ClassName, align: TextAlign) -> Rule {
	Rule::new()
		.with_selector(Selector::class(class))
		.with_value(common_props::TextAlignProp, align)
}

/// A font-size utility rule for `class`.
pub fn text_size(class: ClassName, size: impl Into<Token>) -> Rule {
	Rule::new()
		.with_selector(Selector::class(class))
		.with_token(common_props::FontSize, size).unwrap()
}
