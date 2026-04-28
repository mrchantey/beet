//! Material Design 3 component rules.
//!
//! Provides CSS classes for common MD3 components like buttons, cards,
//! and layout helpers. These rules reference the material design tokens
//! defined in the parent module.

#![cfg_attr(rustfmt, rustfmt_skip)]

use crate::{style::*, token};
use crate::style::material::*;
use crate::token::TokenStore;

// ── Buttons ───────────────────────────────────────────────────────────────────

/// Filled button - the primary action button with high emphasis.
///
/// Uses primary color background with on-primary text.
pub fn button_filled() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("btn-filled"))
		.with_token::<common_props::BackgroundColor>(colors::Primary)
		.with_token::<common_props::ForegroundColor>(colors::OnPrimary)
		.with_token::<TypographyProps>(typography::LabelLarge)
		.with_token::<ShapeProps>(geometry::ShapeFull)
		.with_token::<common_props::ElevationProp>(geometry::Elevation0)
}

/// Outlined button - medium emphasis with visible border.
///
/// Transparent background with outline border.
pub fn button_outlined() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("btn-outlined"))
		.with_token::<common_props::ForegroundColor>(colors::Primary)
		.with_token::<TypographyProps>(typography::LabelLarge)
		.with_token::<ShapeProps>(geometry::ShapeFull)
		.with_token::<common_props::ElevationProp>(geometry::Elevation0)
}

/// Text button - lowest emphasis, no container.
///
/// Transparent background, colored text only.
pub fn button_text() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("btn-text"))
		.with_token::<common_props::ForegroundColor>(colors::Primary)
		.with_token::<TypographyProps>(typography::LabelLarge)
		.with_token::<ShapeProps>(geometry::ShapeFull)
}

/// Tonal button - medium emphasis with secondary container color.
///
/// Uses secondary container for subtle emphasis.
pub fn button_tonal() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("btn-tonal"))
		.with_token::<common_props::BackgroundColor>(colors::SecondaryContainer)
		.with_token::<common_props::ForegroundColor>(colors::OnSecondaryContainer)
		.with_token::<TypographyProps>(typography::LabelLarge)
		.with_token::<ShapeProps>(geometry::ShapeFull)
		.with_token::<common_props::ElevationProp>(geometry::Elevation0)
}

/// Elevated button - medium emphasis with shadow elevation.
///
/// Surface background with subtle elevation shadow.
pub fn button_elevated() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("btn-elevated"))
		.with_token::<common_props::BackgroundColor>(colors::Surface)
		.with_token::<common_props::ForegroundColor>(colors::Primary)
		.with_token::<TypographyProps>(typography::LabelLarge)
		.with_token::<ShapeProps>(geometry::ShapeFull)
		.with_token::<common_props::ElevationProp>(geometry::Elevation1)
}

/// Generic button base styles.
///
/// Applied to all `<button>` elements for consistent baseline styling.
pub fn button_base() -> Rule {
	Rule::new()
		.with_predicate(Predicate::Tag("button".into()))
		.with_token::<TypographyProps>(typography::LabelLarge)
		.with_token::<ShapeProps>(geometry::ShapeMedium)
}

// ── Cards ─────────────────────────────────────────────────────────────────────

/// Filled card - container with the highest surface elevation.
///
/// Uses surface-container-highest background, no shadow.
pub fn card_filled() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("card-filled"))
		.with_token::<common_props::BackgroundColor>(colors::SurfaceContainerHighest)
		.with_token::<common_props::ForegroundColor>(colors::OnSurface)
		.with_token::<ShapeProps>(geometry::ShapeMedium)
		.with_token::<common_props::ElevationProp>(geometry::Elevation0)
}

/// Elevated card - container with shadow elevation.
///
/// Surface container with level 1 shadow for subtle lift.
pub fn card_elevated() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("card-elevated"))
		.with_token::<common_props::BackgroundColor>(colors::SurfaceContainerLow)
		.with_token::<common_props::ForegroundColor>(colors::OnSurface)
		.with_token::<ShapeProps>(geometry::ShapeMedium)
		.with_token::<common_props::ElevationProp>(geometry::Elevation1)
}

/// Outlined card - container with visible border.
///
/// Surface background with outline border, no shadow.
pub fn card_outlined() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("card-outlined"))
		.with_token::<common_props::BackgroundColor>(colors::Surface)
		.with_token::<common_props::ForegroundColor>(colors::OnSurface)
		.with_token::<ShapeProps>(geometry::ShapeMedium)
		.with_token::<common_props::ElevationProp>(geometry::Elevation0)
}

// ── Typography Utility Classes ────────────────────────────────────────────────

/// Display large - largest hero text.
pub fn text_display_large() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-display-large"))
		.with_token::<TypographyProps>(typography::DisplayLarge)
}

/// Display medium - medium hero text.
pub fn text_display_medium() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-display-medium"))
		.with_token::<TypographyProps>(typography::DisplayMedium)
}

/// Display small - small hero text.
pub fn text_display_small() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-display-small"))
		.with_token::<TypographyProps>(typography::DisplaySmall)
}

/// Headline large - large section heading.
pub fn text_headline_large() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-headline-large"))
		.with_token::<TypographyProps>(typography::HeadlineLarge)
}

/// Headline medium - medium section heading.
pub fn text_headline_medium() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-headline-medium"))
		.with_token::<TypographyProps>(typography::HeadlineMedium)
}

/// Headline small - small section heading.
pub fn text_headline_small() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-headline-small"))
		.with_token::<TypographyProps>(typography::HeadlineSmall)
}

/// Title large - large title text.
pub fn text_title_large() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-title-large"))
		.with_token::<TypographyProps>(typography::TitleLarge)
}

/// Title medium - medium title text.
pub fn text_title_medium() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-title-medium"))
		.with_token::<TypographyProps>(typography::TitleMedium)
}

/// Title small - small title text.
pub fn text_title_small() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-title-small"))
		.with_token::<TypographyProps>(typography::TitleSmall)
}

/// Body large - large body text.
pub fn text_body_large() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-body-large"))
		.with_token::<TypographyProps>(typography::BodyLarge)
}

/// Body medium - medium body text (default).
pub fn text_body_medium() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-body-medium"))
		.with_token::<TypographyProps>(typography::BodyMedium)
}

/// Body small - small body text.
pub fn text_body_small() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-body-small"))
		.with_token::<TypographyProps>(typography::BodySmall)
}

/// Label large - large label text.
pub fn text_label_large() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-label-large"))
		.with_token::<TypographyProps>(typography::LabelLarge)
}

/// Label medium - medium label text.
pub fn text_label_medium() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-label-medium"))
		.with_token::<TypographyProps>(typography::LabelMedium)
}

/// Label small - small label text.
pub fn text_label_small() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("text-label-small"))
		.with_token::<TypographyProps>(typography::LabelSmall)
}

// ── Color Utility Classes ─────────────────────────────────────────────────────

/// Primary color scheme - primary background with on-primary text.
pub fn color_primary() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("color-primary"))
		.with_token::<ColorRoleProps>(colors::PrimaryRole)
}

// ── Shape Utility Classes ─────────────────────────────────────────────────────

/// No border radius.
pub fn shape_none() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("shape-none"))
		.with_token::<ShapeProps>(geometry::ShapeNone)
}

/// Extra small border radius (4px).
pub fn shape_extra_small() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("shape-xs"))
		.with_token::<ShapeProps>(geometry::ShapeExtraSmall)
}

/// Small border radius (8px).
pub fn shape_small() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("shape-sm"))
		.with_token::<ShapeProps>(geometry::ShapeSmall)
}

/// Medium border radius (12px).
pub fn shape_medium() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("shape-md"))
		.with_token::<ShapeProps>(geometry::ShapeMedium)
}

/// Large border radius (16px).
pub fn shape_large() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("shape-lg"))
		.with_token::<ShapeProps>(geometry::ShapeLarge)
}

/// Extra large border radius (28px).
pub fn shape_extra_large() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("shape-xl"))
		.with_token::<ShapeProps>(geometry::ShapeExtraLarge)
}

/// Full border radius (pill/circular).
pub fn shape_full() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("shape-full"))
		.with_token::<ShapeProps>(geometry::ShapeFull)
}

// ── Elevation Utility Classes ─────────────────────────────────────────────────

/// No elevation shadow.
pub fn elevation_0() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("elevation-0"))
		.with_token::<common_props::ElevationProp>(geometry::Elevation0)
}

/// Level 1 elevation shadow.
pub fn elevation_1() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("elevation-1"))
		.with_token::<common_props::ElevationProp>(geometry::Elevation1)
}

/// Level 2 elevation shadow.
pub fn elevation_2() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("elevation-2"))
		.with_token::<common_props::ElevationProp>(geometry::Elevation2)
}

/// Level 3 elevation shadow.
pub fn elevation_3() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("elevation-3"))
		.with_token::<common_props::ElevationProp>(geometry::Elevation3)
}

/// Level 4 elevation shadow.
pub fn elevation_4() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("elevation-4"))
		.with_token::<common_props::ElevationProp>(geometry::Elevation4)
}

/// Level 5 elevation shadow.
pub fn elevation_5() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("elevation-5"))
		.with_token::<common_props::ElevationProp>(geometry::Elevation5)
}

// ── Layout Components ─────────────────────────────────────────────────────────

/// App bar / header - surface background suitable for navigation.
///
/// 64px height with surface background and elevation for scrolled state.
pub fn app_bar() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("app-bar"))
		.with_token::<common_props::BackgroundColor>(colors::Surface)
		.with_token::<common_props::ForegroundColor>(colors::OnSurface)
		.with_token::<common_props::ElevationProp>(geometry::Elevation0)
}

/// App bar in scrolled state - adds elevation shadow.
pub fn app_bar_scrolled() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("app-bar-scrolled"))
		.with_token::<common_props::BackgroundColor>(colors::SurfaceContainer)
		.with_token::<common_props::ElevationProp>(geometry::Elevation2)
}

/// Container - basic surface container for grouping content.
pub fn container() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("container"))
		.with_token::<common_props::BackgroundColor>(colors::SurfaceContainer)
		.with_token::<common_props::ForegroundColor>(colors::OnSurface)
}

/// Page - full page background using the base surface color.
pub fn page() -> Rule {
	Rule::new()
		.with_predicate(Predicate::class("page"))
		.with_token::<common_props::BackgroundColor>(colors::Surface)
		.with_token::<common_props::ForegroundColor>(colors::OnSurface)
		.with_token::<TypographyProps>(typography::BodyMedium)
}


token!(ButtonBase, Rule);
token!(ButtonFilled, Rule);
token!(ButtonOutlined, Rule);
token!(ButtonText, Rule);
token!(ButtonTonal, Rule);
token!(ButtonElevated, Rule);
token!(CardFilled, Rule);
token!(CardElevated, Rule);
token!(CardOutlined, Rule);
token!(TextDisplayLarge, Rule);
token!(TextDisplayMedium, Rule);
token!(TextDisplaySmall, Rule);
token!(TextHeadlineLarge, Rule);
token!(TextHeadlineMedium, Rule);
token!(TextHeadlineSmall, Rule);
token!(TextTitleLarge, Rule);
token!(TextTitleMedium, Rule);
token!(TextTitleSmall, Rule);
token!(TextBodyLarge, Rule);
token!(TextBodyMedium, Rule);
token!(TextBodySmall, Rule);
token!(TextLabelLarge, Rule);
token!(TextLabelMedium, Rule);
token!(TextLabelSmall, Rule);
token!(ColorPrimary, Rule);
token!(ShapeNone, Rule);
token!(ShapeExtraSmall, Rule);
token!(ShapeSmall, Rule);
token!(ShapeMedium, Rule);
token!(ShapeLarge, Rule);
token!(ShapeExtraLarge, Rule);
token!(ShapeFull, Rule);
token!(Elevation0, Rule);
token!(Elevation1, Rule);
token!(Elevation2, Rule);
token!(Elevation3, Rule);
token!(Elevation4, Rule);
token!(Elevation5, Rule);
token!(AppBar, Rule);
token!(AppBarScrolled, Rule);
token!(Container, Rule);
token!(Page, Rule);



// ── Public API ────────────────────────────────────────────────────────────────

/// Returns all Material Design component rules as a [`TokenStore`].
///
/// This includes buttons, cards, typography utilities, color utilities,
/// shape utilities, elevation utilities, and layout components.
pub fn all_rules() -> TokenStore {
	TokenStore::default()
		.with_value(ButtonBase, button_base()).unwrap()
		.with_value(ButtonFilled, button_filled()).unwrap()
		.with_value(ButtonOutlined, button_outlined()).unwrap()
		.with_value(ButtonText, button_text()).unwrap()
		.with_value(ButtonTonal, button_tonal()).unwrap()
		.with_value(ButtonElevated, button_elevated()).unwrap()
		.with_value(CardFilled, card_filled()).unwrap()
		.with_value(CardElevated, card_elevated()).unwrap()
		.with_value(CardOutlined, card_outlined()).unwrap()
		.with_value(TextDisplayLarge, text_display_large()).unwrap()
		.with_value(TextDisplayMedium, text_display_medium()).unwrap()
		.with_value(TextDisplaySmall, text_display_small()).unwrap()
		.with_value(TextHeadlineLarge, text_headline_large()).unwrap()
		.with_value(TextHeadlineMedium, text_headline_medium()).unwrap()
		.with_value(TextHeadlineSmall, text_headline_small()).unwrap()
		.with_value(TextTitleLarge, text_title_large()).unwrap()
		.with_value(TextTitleMedium, text_title_medium()).unwrap()
		.with_value(TextTitleSmall, text_title_small()).unwrap()
		.with_value(TextBodyLarge, text_body_large()).unwrap()
		.with_value(TextBodyMedium, text_body_medium()).unwrap()
		.with_value(TextBodySmall, text_body_small()).unwrap()
		.with_value(TextLabelLarge, text_label_large()).unwrap()
		.with_value(TextLabelMedium, text_label_medium()).unwrap()
		.with_value(TextLabelSmall, text_label_small()).unwrap()
		.with_value(ColorPrimary, color_primary()).unwrap()
		.with_value(ShapeNone, shape_none()).unwrap()
		.with_value(ShapeExtraSmall, shape_extra_small()).unwrap()
		.with_value(ShapeSmall, shape_small()).unwrap()
		.with_value(ShapeMedium, shape_medium()).unwrap()
		.with_value(ShapeLarge, shape_large()).unwrap()
		.with_value(ShapeExtraLarge, shape_extra_large()).unwrap()
		.with_value(ShapeFull, shape_full()).unwrap()
		.with_value(Elevation0, elevation_0()).unwrap()
		.with_value(Elevation1, elevation_1()).unwrap()
		.with_value(Elevation2, elevation_2()).unwrap()
		.with_value(Elevation3, elevation_3()).unwrap()
		.with_value(Elevation4, elevation_4()).unwrap()
		.with_value(Elevation5, elevation_5()).unwrap()
		.with_value(AppBar, app_bar()).unwrap()
		.with_value(AppBarScrolled, app_bar_scrolled()).unwrap()
		.with_value(Container, container()).unwrap()
		.with_value(Page, page()).unwrap()
}
