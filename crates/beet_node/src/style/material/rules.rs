//! Material Design 3 component rules.
//!
//! Provides CSS classes for common MD3 components like buttons, cards,
//! and layout helpers. These rules reference the material design tokens
//! defined in the parent module.

#![cfg_attr(rustfmt, rustfmt_skip)]

use crate::style::*;
use crate::style::material::*;

// ── Buttons ───────────────────────────────────────────────────────────────────

/// Filled button - the primary action button with high emphasis.
///
/// Uses primary color background with on-primary text.
pub fn button_filled() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("btn-filled"))
		.with_token::<common_props::BackgroundColor, colors::Primary>()
		.with_token::<common_props::ForegroundColor, colors::OnPrimary>()
		.with_token::<TypographyProps, typography::LabelLarge>()
		.with_token::<ShapeProps, geometry::ShapeFull>()
		.with_token::<common_props::ElevationProp, geometry::Elevation0>()
}

/// Outlined button - medium emphasis with visible border.
///
/// Transparent background with outline border.
pub fn button_outlined() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("btn-outlined"))
		.with_token::<common_props::ForegroundColor, colors::Primary>()
		.with_token::<TypographyProps, typography::LabelLarge>()
		.with_token::<ShapeProps, geometry::ShapeFull>()
		.with_token::<common_props::ElevationProp, geometry::Elevation0>()
}

/// Text button - lowest emphasis, no container.
///
/// Transparent background, colored text only.
pub fn button_text() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("btn-text"))
		.with_token::<common_props::ForegroundColor, colors::Primary>()
		.with_token::<TypographyProps, typography::LabelLarge>()
		.with_token::<ShapeProps, geometry::ShapeFull>()
}

/// Tonal button - medium emphasis with secondary container color.
///
/// Uses secondary container for subtle emphasis.
pub fn button_tonal() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("btn-tonal"))
		.with_token::<common_props::BackgroundColor, colors::SecondaryContainer>()
		.with_token::<common_props::ForegroundColor, colors::OnSecondaryContainer>()
		.with_token::<TypographyProps, typography::LabelLarge>()
		.with_token::<ShapeProps, geometry::ShapeFull>()
		.with_token::<common_props::ElevationProp, geometry::Elevation0>()
}

/// Elevated button - medium emphasis with shadow elevation.
///
/// Surface background with subtle elevation shadow.
pub fn button_elevated() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("btn-elevated"))
		.with_token::<common_props::BackgroundColor, colors::Surface>()
		.with_token::<common_props::ForegroundColor, colors::Primary>()
		.with_token::<TypographyProps, typography::LabelLarge>()
		.with_token::<ShapeProps, geometry::ShapeFull>()
		.with_token::<common_props::ElevationProp, geometry::Elevation1>()
}

/// Generic button base styles.
///
/// Applied to all `<button>` elements for consistent baseline styling.
pub fn button_base() -> Rule {
	Rule::new()
		.with_predicate(Predicate::Tag("button".into()))
		.with_token::<TypographyProps, typography::LabelLarge>()
		.with_token::<ShapeProps, geometry::ShapeMedium>()
}

// ── Cards ─────────────────────────────────────────────────────────────────────

/// Filled card - container with the highest surface elevation.
///
/// Uses surface-container-highest background, no shadow.
pub fn card_filled() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("card-filled"))
		.with_token::<common_props::BackgroundColor, colors::SurfaceContainerHighest>()
		.with_token::<common_props::ForegroundColor, colors::OnSurface>()
		.with_token::<ShapeProps, geometry::ShapeMedium>()
		.with_token::<common_props::ElevationProp, geometry::Elevation0>()
}

/// Elevated card - container with shadow elevation.
///
/// Surface container with level 1 shadow for subtle lift.
pub fn card_elevated() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("card-elevated"))
		.with_token::<common_props::BackgroundColor, colors::SurfaceContainerLow>()
		.with_token::<common_props::ForegroundColor, colors::OnSurface>()
		.with_token::<ShapeProps, geometry::ShapeMedium>()
		.with_token::<common_props::ElevationProp, geometry::Elevation1>()
}

/// Outlined card - container with visible border.
///
/// Surface background with outline border, no shadow.
pub fn card_outlined() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("card-outlined"))
		.with_token::<common_props::BackgroundColor, colors::Surface>()
		.with_token::<common_props::ForegroundColor, colors::OnSurface>()
		.with_token::<ShapeProps, geometry::ShapeMedium>()
		.with_token::<common_props::ElevationProp, geometry::Elevation0>()
}

// ── Typography Utility Classes ────────────────────────────────────────────────

/// Display large - largest hero text.
pub fn text_display_large() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-display-large"))
		.with_token::<TypographyProps, typography::DisplayLarge>()
}

/// Display medium - medium hero text.
pub fn text_display_medium() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-display-medium"))
		.with_token::<TypographyProps, typography::DisplayMedium>()
}

/// Display small - small hero text.
pub fn text_display_small() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-display-small"))
		.with_token::<TypographyProps, typography::DisplaySmall>()
}

/// Headline large - large section heading.
pub fn text_headline_large() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-headline-large"))
		.with_token::<TypographyProps, typography::HeadlineLarge>()
}

/// Headline medium - medium section heading.
pub fn text_headline_medium() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-headline-medium"))
		.with_token::<TypographyProps, typography::HeadlineMedium>()
}

/// Headline small - small section heading.
pub fn text_headline_small() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-headline-small"))
		.with_token::<TypographyProps, typography::HeadlineSmall>()
}

/// Title large - large title text.
pub fn text_title_large() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-title-large"))
		.with_token::<TypographyProps, typography::TitleLarge>()
}

/// Title medium - medium title text.
pub fn text_title_medium() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-title-medium"))
		.with_token::<TypographyProps, typography::TitleMedium>()
}

/// Title small - small title text.
pub fn text_title_small() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-title-small"))
		.with_token::<TypographyProps, typography::TitleSmall>()
}

/// Body large - large body text.
pub fn text_body_large() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-body-large"))
		.with_token::<TypographyProps, typography::BodyLarge>()
}

/// Body medium - medium body text (default).
pub fn text_body_medium() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-body-medium"))
		.with_token::<TypographyProps, typography::BodyMedium>()
}

/// Body small - small body text.
pub fn text_body_small() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-body-small"))
		.with_token::<TypographyProps, typography::BodySmall>()
}

/// Label large - large label text.
pub fn text_label_large() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-label-large"))
		.with_token::<TypographyProps, typography::LabelLarge>()
}

/// Label medium - medium label text.
pub fn text_label_medium() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-label-medium"))
		.with_token::<TypographyProps, typography::LabelMedium>()
}

/// Label small - small label text.
pub fn text_label_small() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-label-small"))
		.with_token::<TypographyProps, typography::LabelSmall>()
}

// ── Color Utility Classes ─────────────────────────────────────────────────────

/// Primary color scheme - primary background with on-primary text.
pub fn color_primary() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("color-primary"))
		.with_token::<ColorRoleProps, colors::PrimaryRole>()
}

// ── Shape Utility Classes ─────────────────────────────────────────────────────

/// No border radius.
pub fn shape_none() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("shape-none"))
		.with_token::<ShapeProps, geometry::ShapeNone>()
}

/// Extra small border radius (4px).
pub fn shape_extra_small() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("shape-xs"))
		.with_token::<ShapeProps, geometry::ShapeExtraSmall>()
}

/// Small border radius (8px).
pub fn shape_small() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("shape-sm"))
		.with_token::<ShapeProps, geometry::ShapeSmall>()
}

/// Medium border radius (12px).
pub fn shape_medium() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("shape-md"))
		.with_token::<ShapeProps, geometry::ShapeMedium>()
}

/// Large border radius (16px).
pub fn shape_large() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("shape-lg"))
		.with_token::<ShapeProps, geometry::ShapeLarge>()
}

/// Extra large border radius (28px).
pub fn shape_extra_large() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("shape-xl"))
		.with_token::<ShapeProps, geometry::ShapeExtraLarge>()
}

/// Full border radius (pill/circular).
pub fn shape_full() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("shape-full"))
		.with_token::<ShapeProps, geometry::ShapeFull>()
}

// ── Elevation Utility Classes ─────────────────────────────────────────────────

/// No elevation shadow.
pub fn elevation_0() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("elevation-0"))
		.with_token::<common_props::ElevationProp, geometry::Elevation0>()
}

/// Level 1 elevation shadow.
pub fn elevation_1() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("elevation-1"))
		.with_token::<common_props::ElevationProp, geometry::Elevation1>()
}

/// Level 2 elevation shadow.
pub fn elevation_2() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("elevation-2"))
		.with_token::<common_props::ElevationProp, geometry::Elevation2>()
}

/// Level 3 elevation shadow.
pub fn elevation_3() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("elevation-3"))
		.with_token::<common_props::ElevationProp, geometry::Elevation3>()
}

/// Level 4 elevation shadow.
pub fn elevation_4() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("elevation-4"))
		.with_token::<common_props::ElevationProp, geometry::Elevation4>()
}

/// Level 5 elevation shadow.
pub fn elevation_5() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("elevation-5"))
		.with_token::<common_props::ElevationProp, geometry::Elevation5>()
}

// ── Layout Components ─────────────────────────────────────────────────────────

/// App bar / header - surface background suitable for navigation.
///
/// 64px height with surface background and elevation for scrolled state.
pub fn app_bar() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("app-bar"))
		.with_token::<common_props::BackgroundColor, colors::Surface>()
		.with_token::<common_props::ForegroundColor, colors::OnSurface>()
		.with_token::<common_props::ElevationProp, geometry::Elevation0>()
}

/// App bar in scrolled state - adds elevation shadow.
pub fn app_bar_scrolled() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("app-bar-scrolled"))
		.with_token::<common_props::BackgroundColor, colors::SurfaceContainer>()
		.with_token::<common_props::ElevationProp, geometry::Elevation2>()
}

/// Container - basic surface container for grouping content.
pub fn container() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("container"))
		.with_token::<common_props::BackgroundColor, colors::SurfaceContainer>()
		.with_token::<common_props::ForegroundColor, colors::OnSurface>()
}

/// Page - full page background using the base surface color.
pub fn page() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("page"))
		.with_token::<common_props::BackgroundColor, colors::Surface>()
		.with_token::<common_props::ForegroundColor, colors::OnSurface>()
		.with_token::<TypographyProps, typography::BodyMedium>()
}


// ── Public API ────────────────────────────────────────────────────────────────

/// Returns all Material Design component rules as a [`Vec`].
///
/// This includes buttons, cards, typography utilities, color utilities,
/// shape utilities, elevation utilities, and layout components.
pub fn all_rules() -> Vec<Rule> {
	vec![
		// Buttons
		button_base(),
		button_filled(),
		button_outlined(),
		button_text(),
		button_tonal(),
		button_elevated(),
		// Cards
		card_filled(),
		card_elevated(),
		card_outlined(),
		// Typography
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
		// Colors
		color_primary(),
		// Shapes
		shape_none(),
		shape_extra_small(),
		shape_small(),
		shape_medium(),
		shape_large(),
		shape_extra_large(),
		shape_full(),
		// Elevation
		elevation_0(),
		elevation_1(),
		elevation_2(),
		elevation_3(),
		elevation_4(),
		elevation_5(),
		// Layout
		app_bar(),
		app_bar_scrolled(),
		container(),
		page(),
	]
}
