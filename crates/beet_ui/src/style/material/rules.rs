//! Material Design 3 component rules.
//!
//! Provides CSS classes for common MD3 components like buttons, cards,
//! and layout helpers. These rules reference the material design tokens
//! defined in the parent module.
#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::style::*;
use crate::prelude::*;
use crate::style::material::*;
// the class-name vocabulary these rules style lives in `token::classes`, shared
// with the widgets that emit the same classes.
use crate::token::classes::*;
use beet_core::prelude::Duration;

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

/// Class-based button baseline, mirroring [`button_base`].
///
/// Lets a non-`<button>` element styled as a button (eg an `<a>` [`Link`]) pick
/// up the same baseline typography and shape via the `.btn` class.
pub fn button_class() -> Rule {
	Rule::new()
		.with_selector(Selector::class(BTN))
		.with_token(TypographyProps,typography::LabelLarge).unwrap()
		.with_token(ShapeProps,geometry::ShapeMedium).unwrap()
}

/// Secondary filled button - medium emphasis using the secondary color.
pub fn button_secondary() -> Rule {
	Rule::new()
		.with_selector(Selector::class(BTN_SECONDARY))
		.with_token(common_props::BackgroundColor,colors::Secondary).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnSecondary).unwrap()
		.with_token(TypographyProps,typography::LabelLarge).unwrap()
		.with_token(ShapeProps,geometry::ShapeFull).unwrap()
		.with_token(common_props::ElevationProp,geometry::Elevation0).unwrap()
}

/// Tertiary filled button - medium emphasis using the tertiary color.
pub fn button_tertiary() -> Rule {
	Rule::new()
		.with_selector(Selector::class(BTN_TERTIARY))
		.with_token(common_props::BackgroundColor,colors::Tertiary).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnTertiary).unwrap()
		.with_token(TypographyProps,typography::LabelLarge).unwrap()
		.with_token(ShapeProps,geometry::ShapeFull).unwrap()
		.with_token(common_props::ElevationProp,geometry::Elevation0).unwrap()
}

/// Error button - destructive action using the error color.
pub fn button_error() -> Rule {
	Rule::new()
		.with_selector(Selector::class(BTN_ERROR))
		.with_token(common_props::BackgroundColor,colors::Error).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnError).unwrap()
		.with_token(TypographyProps,typography::LabelLarge).unwrap()
		.with_token(ShapeProps,geometry::ShapeFull).unwrap()
		.with_token(common_props::ElevationProp,geometry::Elevation0).unwrap()
}

/// Icon button - circular, container-less button sized for a single glyph.
pub fn button_icon() -> Rule {
	Rule::new()
		.with_selector(Selector::class(BTN_ICON))
		.with_token(common_props::ForegroundColor,colors::OnSurfaceVariant).unwrap()
		.with_token(ShapeProps,geometry::ShapeFull).unwrap()
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

/// App bar / header - elevated surface suitable for navigation.
///
/// Reads as raised against the page: a `SurfaceContainer` fill plus a bottom
/// divider, the terminal stand-in for the web app bar's box-shadow elevation.
pub fn app_bar() -> Rule {
	Rule::new()
		.with_selector(Selector::class(APP_BAR))
		.with_token(common_props::BackgroundColor,colors::SurfaceContainer).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
		.with_token(common_props::ElevationProp,geometry::Elevation0).unwrap()
		.with_token(common_props::BorderColorProp,colors::OutlineVariant).unwrap()
		.with_token(common_props::BorderBottomWidth,geometry::OutlineWidthThin).unwrap()
		// title and nav sit side by side, spread across the bar
		.with_value(common_props::DisplayProp, Display::Flex)
		.with_value(common_props::AlignItemsProp, AlignItems::Center)
		.with_value(common_props::JustifyContentProp, JustifyContent::SpaceBetween)
		.with_value(common_props::ColumnGapProp, 2u32)
}

/// App bar navigation - a flex row so its links are spaced rather than running
/// together as adjacent inline anchors.
pub fn app_bar_nav() -> Rule {
	Rule::new()
		.with_selector(Selector::class(APP_BAR_NAV))
		.with_value(common_props::DisplayProp, Display::Flex)
		.with_value(common_props::ColumnGapProp, 2u32)
}

/// Page `<footer>` - mirrors the app bar's elevation with a top divider.
pub fn footer() -> Rule {
	Rule::new()
		.with_selector(Selector::tag("footer"))
		.with_token(common_props::BackgroundColor,colors::SurfaceContainer).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnSurfaceVariant).unwrap()
		.with_token(common_props::BorderColorProp,colors::OutlineVariant).unwrap()
		.with_token(common_props::BorderTopWidth,geometry::OutlineWidthThin).unwrap()
}

/// App bar in scrolled state - adds elevation shadow.
pub fn app_bar_scrolled() -> Rule {
	Rule::new()
		.with_selector(Selector::class(APP_BAR_SCROLLED))
		.with_token(common_props::BackgroundColor,colors::SurfaceContainer).unwrap()
		.with_token(common_props::ElevationProp,geometry::Elevation2).unwrap()
}

/// Container - the body's sidebar + main row.
///
/// A flex row (default [`Direction::Horizontal`]) so the `nav` sidebar and
/// `<main>` content sit side by side rather than stacking.
pub fn container() -> Rule {
	Rule::new()
		.with_selector(Selector::class(CONTAINER))
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
		.with_value(common_props::DisplayProp, Display::Flex)
		// stretch the sidebar to the row height so its right divider runs the
		// full height of the content, not just its own entries.
		.with_value(common_props::AlignItemsProp, AlignItems::Stretch)
}

/// Main content column - grows to fill the space beside the sidebar.
pub fn main_content() -> Rule {
	Rule::new()
		.with_selector(Selector::tag("main"))
		.with_value(common_props::FlexGrowProp, 1u32)
		.with_value(common_props::Padding, Spacing::all(Length::Rem(1.)))
}

/// Page - full page background using the base surface color.
pub fn page() -> Rule {
	Rule::new()
		.with_selector(Selector::class(PAGE))
		.with_token(common_props::BackgroundColor,colors::Surface).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
		.with_token(TypographyProps,typography::BodyMedium).unwrap()
}

// ── Form controls ───────────────────────────────────────────────────────────

/// Shared baseline for `.input` text fields and text areas.
pub fn input_base() -> Rule {
	Rule::new()
		.with_selector(Selector::class(INPUT))
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
		.with_token(TypographyProps,typography::BodyLarge).unwrap()
		.with_token(ShapeProps,geometry::ShapeExtraSmall).unwrap()
		.with_value(common_props::Padding, Spacing::all(Length::Rem(0.5)))
}

/// Outlined input - visible border, transparent fill.
pub fn input_outlined() -> Rule {
	Rule::new()
		.with_selector(Selector::class(INPUT_OUTLINED))
		.with_token(common_props::BorderColorProp,colors::Outline).unwrap()
		.with_token(common_props::OutlineWidth,geometry::OutlineWidthThin).unwrap()
}

/// Filled input - shaded container, no border.
pub fn input_filled() -> Rule {
	Rule::new()
		.with_selector(Selector::class(INPUT_FILLED))
		.with_token(common_props::BackgroundColor,colors::SurfaceContainerHighest).unwrap()
}

/// Text input - lowest emphasis, underline only (no container).
pub fn input_text() -> Rule {
	Rule::new()
		.with_selector(Selector::class(INPUT_TEXT))
		.with_token(common_props::BorderColorProp,colors::OutlineVariant).unwrap()
}

/// Focused input - primary-colored border. Compound selector `.input:focus`.
pub fn input_focus() -> Rule {
	Rule::new()
		.with_selector(Selector::AllOf(vec![
			Selector::class(INPUT),
			Selector::state(ElementState::Focused),
		]))
		.with_token(common_props::BorderColorProp,colors::Primary).unwrap()
}

/// Shared baseline for `.select` elements.
pub fn select_base() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SELECT))
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
		.with_token(TypographyProps,typography::BodyLarge).unwrap()
		.with_token(ShapeProps,geometry::ShapeExtraSmall).unwrap()
		.with_value(common_props::Padding, Spacing::all(Length::Rem(0.5)))
}

/// Outlined select - visible border, transparent fill.
pub fn select_outlined() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SELECT_OUTLINED))
		.with_token(common_props::BorderColorProp,colors::Outline).unwrap()
		.with_token(common_props::OutlineWidth,geometry::OutlineWidthThin).unwrap()
}

/// Filled select - shaded container, no border.
pub fn select_filled() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SELECT_FILLED))
		.with_token(common_props::BackgroundColor,colors::SurfaceContainerHighest).unwrap()
}

/// Text select - lowest emphasis.
pub fn select_text() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SELECT_TEXT))
		.with_token(common_props::BorderColorProp,colors::OutlineVariant).unwrap()
}

/// Focused select - primary-colored border. Compound selector `.select:focus`.
pub fn select_focus() -> Rule {
	Rule::new()
		.with_selector(Selector::AllOf(vec![
			Selector::class(SELECT),
			Selector::state(ElementState::Focused),
		]))
		.with_token(common_props::BorderColorProp,colors::Primary).unwrap()
}

/// Error message text, ie validation feedback below an input.
pub fn error_text() -> Rule {
	Rule::new()
		.with_selector(Selector::class(ERROR_TEXT))
		.with_token(common_props::ForegroundColor,colors::Error).unwrap()
		.with_token(TypographyProps,typography::BodySmall).unwrap()
}

// ── Table ───────────────────────────────────────────────────────────────────

/// Table container - full-width, surface foreground.
pub fn table() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TABLE))
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
		.with_token(TypographyProps,typography::BodyMedium).unwrap()
		.with_value(common_props::Width, Length::Percent(100.))
}

/// Header cells - medium weight, left aligned, padded, bottom border.
pub fn table_th() -> Rule {
	Rule::new()
		.with_selector(Selector::tag("th"))
		.with_token(common_props::FontWeightProp,typography::WeightMedium).unwrap()
		.with_token(common_props::BorderColorProp,colors::Outline).unwrap()
		.with_value(common_props::TextAlignProp, TextAlign::Left)
		.with_value(common_props::Padding, Spacing::all(Length::Rem(0.5)))
}

/// Body cells - padded, faint divider border.
pub fn table_td() -> Rule {
	Rule::new()
		.with_selector(Selector::tag("td"))
		.with_token(common_props::BorderColorProp,colors::OutlineVariant).unwrap()
		.with_value(common_props::Padding, Spacing::all(Length::Rem(0.5)))
}

// ── Disclosure (`<details>`) + sidebar ───────────────────────────────────────

/// Disclosure container - block layout for `<details>`.
pub fn details() -> Rule {
	Rule::new()
		.with_selector(Selector::tag("details"))
		.with_value(common_props::DisplayProp, Display::Block)
}

/// Disclosure header (`<summary>`) - medium weight, surface foreground.
pub fn summary() -> Rule {
	Rule::new()
		.with_selector(Selector::tag("summary"))
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
		.with_token(common_props::FontWeightProp,typography::WeightMedium).unwrap()
}

/// Sidebar nav container - a left rail divided from the main column by a
/// right border, with padding so its links clear the divider.
pub fn sidebar() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SIDEBAR))
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
		.with_token(TypographyProps,typography::BodyMedium).unwrap()
		.with_token(common_props::BorderColorProp,colors::OutlineVariant).unwrap()
		.with_token(common_props::BorderRightWidth,geometry::OutlineWidthThin).unwrap()
		.with_value(common_props::Padding, Spacing {
			right: Length::Rem(1.),
			..Spacing::DEFAULT
		})
}

/// Sidebar link - primary-colored, for navigable leaves and branches.
pub fn sidebar_link() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SIDEBAR_LINK))
		.with_token(common_props::ForegroundColor,colors::Primary).unwrap()
}

/// Nested sidebar item - indented under its parent group. Each nesting level's
/// padding insets the level below it, so the tree steps in per depth. Only the
/// non-root `sidebar-item` carries it; `sidebar-item-root` stays flush left.
pub fn sidebar_item() -> Rule {
	Rule::new()
		.with_selector(Selector::class("sidebar-item"))
		.with_value(common_props::Padding, Spacing {
			left: Length::Rem(1.),
			..Spacing::DEFAULT
		})
}

/// Sidebar group label - faint, for non-navigable headers.
pub fn sidebar_label() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SIDEBAR_LABEL))
		.with_token(common_props::ForegroundColor,colors::OnSurfaceVariant).unwrap()
		.with_token(common_props::FontWeightProp,typography::WeightMedium).unwrap()
}

// ── Utility classes ───────────────────────────────────────────────────────────

/// `display: none` - removed from layout.
pub fn hidden() -> Rule {
	Rule::new()
		.with_selector(Selector::class(HIDDEN))
		.with_value(common_props::DisplayProp, Display::None)
}

/// Hides an element when printing (`@media print { display: none }`).
///
/// Emitted by `Sidebar`/`Header`/`Footer` so chrome drops out of print output.
pub fn print_hidden() -> Rule {
	Rule::new()
		.with_selector(Selector::class(PRINT_HIDDEN))
		.with_media(MediaQuery::Print)
		.with_value(common_props::DisplayProp, Display::None)
}

/// Forces a page break after the element when printing
/// (`@media print { break-after: page }`).
pub fn page_break() -> Rule {
	Rule::new()
		.with_selector(Selector::class(PAGE_BREAK))
		.with_media(MediaQuery::Print)
		.with_value(common_props::BreakAfterProp, BreakAfter::Page)
}

/// Zeroes transition/animation duration when the user prefers reduced motion
/// (`@media (prefers-reduced-motion: reduce) { * { …-duration: 0ms } }`).
pub fn reduced_motion() -> Rule {
	Rule::new()
		.with_selector(Selector::Any)
		.with_media(MediaQuery::ReducedMotion)
		.with_value(common_props::TransitionDurationProp, Duration::ZERO)
		.with_value(common_props::AnimationDurationProp, Duration::ZERO)
}

fn text_align(class: ClassName, align: TextAlign) -> Rule {
	Rule::new()
		.with_selector(Selector::class(class))
		.with_value(common_props::TextAlignProp, align)
}

fn text_size(class: ClassName, size: impl Into<Token>) -> Rule {
	Rule::new()
		.with_selector(Selector::class(class))
		.with_token(common_props::FontSize, size).unwrap()
}

// ── Accessibility ─────────────────────────────────────────────────────────────

/// Focus ring - primary-colored border on any focused element (`:focus`).
pub fn focus_ring() -> Rule {
	Rule::new()
		.with_selector(Selector::state(ElementState::Focused))
		.with_token(common_props::BorderColorProp,colors::Primary).unwrap()
}

/// Disabled elements - faint foreground (`:disabled`).
pub fn disabled_state() -> Rule {
	Rule::new()
		.with_selector(Selector::state(ElementState::Disabled))
		.with_token(common_props::ForegroundColor,colors::OnSurfaceVariant).unwrap()
}

/// Returns all Material Design component rules.
pub fn all_rules() -> Vec<Rule> {
	vec![
		button_base(),
		button_class(),
		button_filled(),
		button_outlined(),
		button_text(),
		button_tonal(),
		button_elevated(),
		button_secondary(),
		button_tertiary(),
		button_error(),
		button_icon(),
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
		app_bar_nav(),
		footer(),
		container(),
		main_content(),
		page(),
		// form controls — state/compound rules first so they win the cascade
		input_focus(),
		select_focus(),
		input_base(),
		input_outlined(),
		input_filled(),
		input_text(),
		select_base(),
		select_outlined(),
		select_filled(),
		select_text(),
		error_text(),
		// table
		table(),
		table_th(),
		table_td(),
		// disclosure + sidebar
		details(),
		summary(),
		sidebar(),
		sidebar_link(),
		sidebar_item(),
		sidebar_label(),
		// utilities
		hidden(),
		text_align(TEXT_LEFT, TextAlign::Left),
		text_align(TEXT_CENTER, TextAlign::Center),
		text_align(TEXT_RIGHT, TextAlign::Right),
		text_size(TEXT_XS, typography::FontSizeLabelSmall),
		text_size(TEXT_SM, typography::FontSizeBodySmall),
		text_size(TEXT_BASE, typography::FontSizeBodyLarge),
		text_size(TEXT_LG, typography::FontSizeTitleLarge),
		text_size(TEXT_XL, typography::FontSizeHeadlineSmall),
		text_size(TEXT_2XL, typography::FontSizeHeadlineMedium),
		// print utilities — gated behind `@media print`
		print_hidden(),
		page_break(),
		// reduced motion — gated behind `@media (prefers-reduced-motion)`
		reduced_motion(),
		// accessibility — global state rules
		focus_ring(),
		disabled_state(),
	]
}


#[cfg(test)]
mod tests {
	use super::*;
	use beet_core::prelude::*;
	use crate::style::material::default_token_map;

	/// CSS map covering both the material tokens and the common props the new
	/// component rules reference.
	fn css_map() -> CssTokenMap {
		default_token_map().with_extend(common_props::token_map())
	}

	#[beet_core::test]
	fn component_rules_css() {
		let rule_set = RuleSet::new(Rule::new()).with_rules(vec![
			error_text(),
			input_base(),
			input_outlined(),
			input_focus(),
			table_th(),
			details(),
			summary(),
			hidden(),
			text_align(TEXT_CENTER, TextAlign::Center),
		]);
		CssBuilder::default()
			.with_minify(false)
			.with_format_variables(FormatVariables::short())
			.build(&css_map(), &rule_set)
			.unwrap()
			.xpect_snapshot();
	}

	#[beet_core::test]
	fn all_rules_emit_selectors() {
		let css = CssBuilder::default()
			.with_format_variables(FormatVariables::short())
			.build(&css_map(), &RuleSet::new(Rule::new()).with_rules(all_rules()))
			.unwrap();
		// compound `.input:focus` exercises Selector::AllOf serialization
		css.as_str()
			.xpect_contains(".input:focus")
			.xpect_contains(".btn")
			.xpect_contains(".btn-error")
			.xpect_contains(".error-text")
			.xpect_contains("details")
			.xpect_contains(".hidden")
			.xpect_contains(".text-center")
			.xpect_contains(":disabled")
			// print utilities serialize wrapped in an `@media print` at-rule
			.xpect_contains("@media print")
			.xpect_contains(".print-hidden")
			.xpect_contains("break-after")
			// reduced-motion serializes wrapped in its own `@media` at-rule
			.xpect_contains("@media (prefers-reduced-motion: reduce)")
			.xpect_contains("transition-duration");
	}

	/// A media-gated rule serializes wrapped in its `@media` at-rule, with the
	/// selector + declaration nested inside the block.
	#[beet_core::test]
	fn print_rule_wraps_in_media_block() {
		let css = CssBuilder::default()
			.with_minify(true)
			.with_format_variables(FormatVariables::short())
			.build(
				&css_map(),
				&RuleSet::new(Rule::new()).with_rules(vec![print_hidden()]),
			)
			.unwrap();
		// `@media print{ .print-hidden { display: none; } }`
		css.as_str()
			.xpect_contains("@media print{")
			.xpect_contains(".print-hidden")
			.xpect_contains("display: none;");
		// the at-rule wraps the selector (appears before it in the output)
		(css.find("@media print").unwrap() < css.find(".print-hidden").unwrap())
			.xpect_true();
	}

	/// The reduced-motion rule serializes wrapped in its `@media` at-rule and
	/// zeroes both transition and animation duration.
	#[beet_core::test]
	fn reduced_motion_wraps_in_media_block() {
		let css = CssBuilder::default()
			.with_minify(true)
			.with_format_variables(FormatVariables::short())
			.build(
				&css_map(),
				&RuleSet::new(Rule::new()).with_rules(vec![reduced_motion()]),
			)
			.unwrap();
		css.as_str()
			.xpect_contains("@media (prefers-reduced-motion: reduce){")
			.xpect_contains("transition-duration: 0ms;")
			.xpect_contains("animation-duration: 0ms;");
	}

	/// Charcell path: a `.error-text` span resolves its foreground through the
	/// cascade to the same color as the `Error` token (light `:root` fallback).
	#[beet_core::test]
	fn error_text_resolves_to_error_color() {
		let mut world = MaterialStylePlugin::world();
		let entity = world
			.spawn(rsx_direct! { <span {Classes::new([ERROR_TEXT])}/> })
			.id();
		world.with_state::<RuleSetQuery, _>(|query| {
			let foreground =
				query.resolve(entity, common_props::ForegroundColor).unwrap();
			let error = query.resolve(entity, colors::Error).unwrap();
			foreground.xpect_eq(error);
		});
	}
}
