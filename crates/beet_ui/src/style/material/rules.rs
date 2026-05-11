//! Material Design 3 component rules.
//!
//! Provides CSS classes for common MD3 components like buttons, cards,
//! and layout helpers. These rules reference the material design tokens
//! defined in the parent module.
#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::style::*;
use crate::prelude::*;
use crate::style::material::*;

// ── Class name constants ──────────────────────────────────────────────────────

pub const BTN_FILLED: &str = "btn-filled";
pub const BTN_OUTLINED: &str = "btn-outlined";
pub const BTN_TEXT: &str = "btn-text";
pub const BTN_TONAL: &str = "btn-tonal";
pub const BTN_ELEVATED: &str = "btn-elevated";
pub const CARD_FILLED: &str = "card-filled";
pub const CARD_ELEVATED: &str = "card-elevated";
pub const CARD_OUTLINED: &str = "card-outlined";
pub const TEXT_DISPLAY_LARGE: &str = "text-display-large";
pub const TEXT_DISPLAY_MEDIUM: &str = "text-display-medium";
pub const TEXT_DISPLAY_SMALL: &str = "text-display-small";
pub const TEXT_HEADLINE_LARGE: &str = "text-headline-large";
pub const TEXT_HEADLINE_MEDIUM: &str = "text-headline-medium";
pub const TEXT_HEADLINE_SMALL: &str = "text-headline-small";
pub const TEXT_TITLE_LARGE: &str = "text-title-large";
pub const TEXT_TITLE_MEDIUM: &str = "text-title-medium";
pub const TEXT_TITLE_SMALL: &str = "text-title-small";
pub const TEXT_BODY_LARGE: &str = "text-body-large";
pub const TEXT_BODY_MEDIUM: &str = "text-body-medium";
pub const TEXT_BODY_SMALL: &str = "text-body-small";
pub const TEXT_LABEL_LARGE: &str = "text-label-large";
pub const TEXT_LABEL_MEDIUM: &str = "text-label-medium";
pub const TEXT_LABEL_SMALL: &str = "text-label-small";
pub const COLOR_PRIMARY: &str = "color-primary";
pub const SHAPE_NONE: &str = "shape-none";
pub const SHAPE_EXTRA_SMALL: &str = "shape-xs";
pub const SHAPE_SMALL: &str = "shape-sm";
pub const SHAPE_MEDIUM: &str = "shape-md";
pub const SHAPE_LARGE: &str = "shape-lg";
pub const SHAPE_EXTRA_LARGE: &str = "shape-xl";
pub const SHAPE_FULL: &str = "shape-full";
pub const ELEVATION_0: &str = "elevation-0";
pub const ELEVATION_1: &str = "elevation-1";
pub const ELEVATION_2: &str = "elevation-2";
pub const ELEVATION_3: &str = "elevation-3";
pub const ELEVATION_4: &str = "elevation-4";
pub const ELEVATION_5: &str = "elevation-5";
pub const APP_BAR: &str = "app-bar";
pub const APP_BAR_SCROLLED: &str = "app-bar-scrolled";
pub const CONTAINER: &str = "container";
pub const PAGE: &str = "page";

// ── Buttons ───────────────────────────────────────────────────────────────────

/// Filled button - the primary action button with high emphasis.
///
/// Uses primary color background with on-primary text.
pub fn button_filled() -> Rule {
	Rule::new()
		.with_selector(Selector::class(BTN_FILLED))
		.with_token(common_props::BackgroundColor,colors::Primary).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnPrimary).unwrap()
		.with_token(TypographyProps,typography::LabelLarge).unwrap()
		.with_token(ShapeProps,geometry::ShapeFull).unwrap()
		.with_token(common_props::ElevationProp,geometry::Elevation0).unwrap()
}

/// Outlined button - medium emphasis with visible border.
///
/// Transparent background with outline border.
pub fn button_outlined() -> Rule {
	Rule::new()
		.with_selector(Selector::class(BTN_OUTLINED))
		.with_token(common_props::ForegroundColor,colors::Primary).unwrap()
		.with_token(TypographyProps,typography::LabelLarge).unwrap()
		.with_token(ShapeProps,geometry::ShapeFull).unwrap()
		.with_token(common_props::ElevationProp,geometry::Elevation0).unwrap()
}

/// Text button - lowest emphasis, no container.
///
/// Transparent background, colored text only.
pub fn button_text() -> Rule {
	Rule::new()
		.with_selector(Selector::class(BTN_TEXT))
		.with_token(common_props::ForegroundColor,colors::Primary).unwrap()
		.with_token(TypographyProps,typography::LabelLarge).unwrap()
		.with_token(ShapeProps,geometry::ShapeFull).unwrap()
}

/// Tonal button - medium emphasis with secondary container color.
///
/// Uses secondary container for subtle emphasis.
pub fn button_tonal() -> Rule {
	Rule::new()
		.with_selector(Selector::class(BTN_TONAL))
		.with_token(common_props::BackgroundColor,colors::SecondaryContainer).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnSecondaryContainer).unwrap()
		.with_token(TypographyProps,typography::LabelLarge).unwrap()
		.with_token(ShapeProps,geometry::ShapeFull).unwrap()
		.with_token(common_props::ElevationProp,geometry::Elevation0).unwrap()
}

/// Elevated button - medium emphasis with shadow elevation.
///
/// Surface background with subtle elevation shadow.
pub fn button_elevated() -> Rule {
	Rule::new()
		.with_selector(Selector::class(BTN_ELEVATED))
		.with_token(common_props::BackgroundColor,colors::Surface).unwrap()
		.with_token(common_props::ForegroundColor,colors::Primary).unwrap()
		.with_token(TypographyProps,typography::LabelLarge).unwrap()
		.with_token(ShapeProps,geometry::ShapeFull).unwrap()
		.with_token(common_props::ElevationProp,geometry::Elevation1).unwrap()
}

/// Generic button base styles.
///
/// Applied to all `<button>` elements for consistent baseline styling.
pub fn button_base() -> Rule {
	Rule::new()
		.with_selector(Selector::Tag("button".into()))
		.with_token(TypographyProps,typography::LabelLarge).unwrap()
		.with_token(ShapeProps,geometry::ShapeMedium).unwrap()
}

// ── Cards ─────────────────────────────────────────────────────────────────────

/// Filled card - container with the highest surface elevation.
///
/// Uses surface-container-highest background, no shadow.
pub fn card_filled() -> Rule {
	Rule::new()
		.with_selector(Selector::class(CARD_FILLED))
		.with_token(common_props::BackgroundColor,colors::SurfaceContainerHighest).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
		.with_token(ShapeProps,geometry::ShapeMedium).unwrap()
		.with_token(common_props::ElevationProp,geometry::Elevation0).unwrap()
}

/// Elevated card - container with shadow elevation.
///
/// Surface container with level 1 shadow for subtle lift.
pub fn card_elevated() -> Rule {
	Rule::new()
		.with_selector(Selector::class(CARD_ELEVATED))
		.with_token(common_props::BackgroundColor,colors::SurfaceContainerLow).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
		.with_token(ShapeProps,geometry::ShapeMedium).unwrap()
		.with_token(common_props::ElevationProp,geometry::Elevation1).unwrap()
}

/// Outlined card - container with visible border.
///
/// Surface background with outline border, no shadow.
pub fn card_outlined() -> Rule {
	Rule::new()
		.with_selector(Selector::class(CARD_OUTLINED))
		.with_token(common_props::BackgroundColor,colors::Surface).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
		.with_token(ShapeProps,geometry::ShapeMedium).unwrap()
		.with_token(common_props::ElevationProp,geometry::Elevation0).unwrap()
}

// ── Typography Utility Classes ────────────────────────────────────────────────

/// Display large - largest hero text.
pub fn text_display_large() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_DISPLAY_LARGE))
		.with_token(TypographyProps,typography::DisplayLarge).unwrap()
}

/// Display medium - medium hero text.
pub fn text_display_medium() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_DISPLAY_MEDIUM))
		.with_token(TypographyProps,typography::DisplayMedium).unwrap()
}

/// Display small - small hero text.
pub fn text_display_small() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_DISPLAY_SMALL))
		.with_token(TypographyProps,typography::DisplaySmall).unwrap()
}

/// Headline large - large section heading.
pub fn text_headline_large() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_HEADLINE_LARGE))
		.with_token(TypographyProps,typography::HeadlineLarge).unwrap()
}

/// Headline medium - medium section heading.
pub fn text_headline_medium() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_HEADLINE_MEDIUM))
		.with_token(TypographyProps,typography::HeadlineMedium).unwrap()
}

/// Headline small - small section heading.
pub fn text_headline_small() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_HEADLINE_SMALL))
		.with_token(TypographyProps,typography::HeadlineSmall).unwrap()
}

/// Title large - large title text.
pub fn text_title_large() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_TITLE_LARGE))
		.with_token(TypographyProps,typography::TitleLarge).unwrap()
}

/// Title medium - medium title text.
pub fn text_title_medium() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_TITLE_MEDIUM))
		.with_token(TypographyProps,typography::TitleMedium).unwrap()
}

/// Title small - small title text.
pub fn text_title_small() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_TITLE_SMALL))
		.with_token(TypographyProps,typography::TitleSmall).unwrap()
}

/// Body large - large body text.
pub fn text_body_large() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_BODY_LARGE))
		.with_token(TypographyProps,typography::BodyLarge).unwrap()
}

/// Body medium - medium body text (default).
pub fn text_body_medium() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_BODY_MEDIUM))
		.with_token(TypographyProps,typography::BodyMedium).unwrap()
}

/// Body small - small body text.
pub fn text_body_small() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_BODY_SMALL))
		.with_token(TypographyProps,typography::BodySmall).unwrap()
}

/// Label large - large label text.
pub fn text_label_large() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_LABEL_LARGE))
		.with_token(TypographyProps,typography::LabelLarge).unwrap()
}

/// Label medium - medium label text.
pub fn text_label_medium() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_LABEL_MEDIUM))
		.with_token(TypographyProps,typography::LabelMedium).unwrap()
}

/// Label small - small label text.
pub fn text_label_small() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TEXT_LABEL_SMALL))
		.with_token(TypographyProps,typography::LabelSmall).unwrap()
}

// ── Color Utility Classes ─────────────────────────────────────────────────────

/// Primary color scheme - primary background with on-primary text.
pub fn color_primary() -> Rule {
	Rule::new()
		.with_selector(Selector::class(COLOR_PRIMARY))
		.with_token(ColorRoleProps,colors::PrimaryRole).unwrap()
}

// ── Shape Utility Classes ─────────────────────────────────────────────────────

/// No border radius.
pub fn shape_none() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SHAPE_NONE))
		.with_token(ShapeProps,geometry::ShapeNone).unwrap()
}

/// Extra small border radius (4px).
pub fn shape_extra_small() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SHAPE_EXTRA_SMALL))
		.with_token(ShapeProps,geometry::ShapeExtraSmall).unwrap()
}

/// Small border radius (8px).
pub fn shape_small() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SHAPE_SMALL))
		.with_token(ShapeProps,geometry::ShapeSmall).unwrap()
}

/// Medium border radius (12px).
pub fn shape_medium() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SHAPE_MEDIUM))
		.with_token(ShapeProps,geometry::ShapeMedium).unwrap()
}

/// Large border radius (16px).
pub fn shape_large() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SHAPE_LARGE))
		.with_token(ShapeProps,geometry::ShapeLarge).unwrap()
}

/// Extra large border radius (28px).
pub fn shape_extra_large() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SHAPE_EXTRA_LARGE))
		.with_token(ShapeProps,geometry::ShapeExtraLarge).unwrap()
}

/// Full border radius (pill/circular).
pub fn shape_full() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SHAPE_FULL))
		.with_token(ShapeProps,geometry::ShapeFull).unwrap()
}

// ── Elevation Utility Classes ─────────────────────────────────────────────────

/// No elevation shadow.
pub fn elevation_0() -> Rule {
	Rule::new()
		.with_selector(Selector::class(ELEVATION_0))
		.with_token(common_props::ElevationProp,geometry::Elevation0).unwrap()
}

/// Level 1 elevation shadow.
pub fn elevation_1() -> Rule {
	Rule::new()
		.with_selector(Selector::class(ELEVATION_1))
		.with_token(common_props::ElevationProp,geometry::Elevation1).unwrap()
}

/// Level 2 elevation shadow.
pub fn elevation_2() -> Rule {
	Rule::new()
		.with_selector(Selector::class(ELEVATION_2))
		.with_token(common_props::ElevationProp,geometry::Elevation2).unwrap()
}

/// Level 3 elevation shadow.
pub fn elevation_3() -> Rule {
	Rule::new()
		.with_selector(Selector::class(ELEVATION_3))
		.with_token(common_props::ElevationProp,geometry::Elevation3).unwrap()
}

/// Level 4 elevation shadow.
pub fn elevation_4() -> Rule {
	Rule::new()
		.with_selector(Selector::class(ELEVATION_4))
		.with_token(common_props::ElevationProp,geometry::Elevation4).unwrap()
}

/// Level 5 elevation shadow.
pub fn elevation_5() -> Rule {
	Rule::new()
		.with_selector(Selector::class(ELEVATION_5))
		.with_token(common_props::ElevationProp,geometry::Elevation5).unwrap()
}

// ── Layout Components ─────────────────────────────────────────────────────────

/// App bar / header - surface background suitable for navigation.
///
/// 64px height with surface background and elevation for scrolled state.
pub fn app_bar() -> Rule {
	Rule::new()
		.with_selector(Selector::class(APP_BAR))
		.with_token(common_props::BackgroundColor,colors::Surface).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
		.with_token(common_props::ElevationProp,geometry::Elevation0).unwrap()
}

/// App bar in scrolled state - adds elevation shadow.
pub fn app_bar_scrolled() -> Rule {
	Rule::new()
		.with_selector(Selector::class(APP_BAR_SCROLLED))
		.with_token(common_props::BackgroundColor,colors::SurfaceContainer).unwrap()
		.with_token(common_props::ElevationProp,geometry::Elevation2).unwrap()
}

/// Container - basic surface container for grouping content.
pub fn container() -> Rule {
	Rule::new()
		.with_selector(Selector::class(CONTAINER))
		.with_token(common_props::BackgroundColor,colors::SurfaceContainer).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
}

/// Page - full page background using the base surface color.
pub fn page() -> Rule {
	Rule::new()
		.with_selector(Selector::class(PAGE))
		.with_token(common_props::BackgroundColor,colors::Surface).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
		.with_token(TypographyProps,typography::BodyMedium).unwrap()
}

/// Returns all Material Design component rules.
pub fn all_rules() -> Vec<Rule> {
	vec![
		button_base(),
		button_filled(),
		button_outlined(),
		button_text(),
		button_tonal(),
		button_elevated(),
		card_filled(),
		card_elevated(),
		card_outlined(),
		text_display_large(),
		text_display_medium(),
		text_display_small(),
		text_headline_large(),
		text_headline_medium(),
		text_headline_small(),
		text_title_large(),
		text_title_medium(),
		text_title_small(),
		text_body_large(),
		text_body_medium(),
		text_body_small(),
		text_label_large(),
		text_label_medium(),
		text_label_small(),
		color_primary(),
		shape_none(),
		shape_extra_small(),
		shape_small(),
		shape_medium(),
		shape_large(),
		shape_extra_large(),
		shape_full(),
		elevation_0(),
		elevation_1(),
		elevation_2(),
		elevation_3(),
		elevation_4(),
		elevation_5(),
		app_bar(),
		app_bar_scrolled(),
		container(),
		page(),
	]
}
