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

/// Padding shared by every button, giving the label room inside its container.
/// Horizontal `1.25rem` reads as the MD3 inset; the vertical `0.4rem` rounds to
/// zero terminal rows so charcell buttons stay a single line.
fn button_padding() -> Spacing {
	Spacing {
		top: Length::Rem(0.4),
		bottom: Length::Rem(0.4),
		left: Length::Rem(1.25),
		right: Length::Rem(1.25),
	}
}

/// The blanket button baseline, matching both `<button>` and the `.btn` class so
/// a non-`<button>` styled as a button (eg an `<a>` [`Link`]) gets the same
/// typography, shape, padding, and pointer cursor. Carries the shared bits so a
/// variant rule only declares what makes it distinct (color, border, elevation),
/// and strips the underline a prose `<a>` would otherwise carry.
///
/// Corners are slightly rounded ([`ShapeSmall`](geometry::ShapeSmall)) rather
/// than a full pill; the [`button_icon`] variant opts back into the circular full
/// radius.
pub fn button_base() -> Rule {
	Rule::new()
		.with_selector(Selector::tag("button").merge_any(Selector::class(BTN)))
		.with_token(TypographyProps,typography::LabelLarge).unwrap()
		.with_token(ShapeProps,geometry::ShapeSmall).unwrap()
		.with_value(common_props::Padding, button_padding())
		.with_value(common_props::CursorProp, Cursor::Pointer)
		.with_canonical(DecorationLine::DEFAULT)
}

/// Filled button - the primary action button with high emphasis.
pub fn button_filled() -> Rule {
	Rule::new()
		.with_selector(Selector::class(BTN_FILLED))
		.with_token(common_props::BackgroundColor,colors::Primary).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnPrimary).unwrap()
		.with_token(common_props::ElevationProp,geometry::Elevation0).unwrap()
}

/// Outlined button - medium emphasis with a visible border, regular foreground.
pub fn button_outlined() -> Rule {
	Rule::new()
		.with_selector(Selector::class(BTN_OUTLINED))
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
		.with_token(common_props::BorderColorProp,colors::Outline).unwrap()
		.with_token(common_props::OutlineWidth,geometry::OutlineWidthThin).unwrap()
		.with_token(common_props::ElevationProp,geometry::Elevation0).unwrap()
}

/// Text button - lowest emphasis, no container, primary-colored text.
pub fn button_text() -> Rule {
	Rule::new()
		.with_selector(Selector::class(BTN_TEXT))
		.with_token(common_props::ForegroundColor,colors::Primary).unwrap()
}

/// Tonal button - medium emphasis with secondary container color.
pub fn button_tonal() -> Rule {
	Rule::new()
		.with_selector(Selector::class(BTN_TONAL))
		.with_token(common_props::BackgroundColor,colors::SecondaryContainer).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnSecondaryContainer).unwrap()
		.with_token(common_props::ElevationProp,geometry::Elevation0).unwrap()
}

/// Elevated button - medium emphasis with shadow elevation, regular foreground.
pub fn button_elevated() -> Rule {
	Rule::new()
		.with_selector(Selector::class(BTN_ELEVATED))
		.with_token(common_props::BackgroundColor,colors::Surface).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
		.with_token(common_props::ElevationProp,geometry::Elevation1).unwrap()
}

/// Secondary filled button - medium emphasis using the secondary color.
pub fn button_secondary() -> Rule {
	Rule::new()
		.with_selector(Selector::class(BTN_SECONDARY))
		.with_token(common_props::BackgroundColor,colors::Secondary).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnSecondary).unwrap()
		.with_token(common_props::ElevationProp,geometry::Elevation0).unwrap()
}

/// Tertiary filled button - medium emphasis using the tertiary color.
pub fn button_tertiary() -> Rule {
	Rule::new()
		.with_selector(Selector::class(BTN_TERTIARY))
		.with_token(common_props::BackgroundColor,colors::Tertiary).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnTertiary).unwrap()
		.with_token(common_props::ElevationProp,geometry::Elevation0).unwrap()
}

/// Error button - destructive action using the error color.
pub fn button_error() -> Rule {
	Rule::new()
		.with_selector(Selector::class(BTN_ERROR))
		.with_token(common_props::BackgroundColor,colors::Error).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnError).unwrap()
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
		.with_value(common_props::Padding, Spacing::all(Length::Rem(1.)))
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
		.with_value(common_props::Padding, Spacing::all(Length::Rem(1.)))
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
		.with_token(common_props::BorderColorProp,colors::OutlineVariant).unwrap()
		.with_token(common_props::OutlineWidth,geometry::OutlineWidthThin).unwrap()
		.with_value(common_props::Padding, Spacing::all(Length::Rem(1.)))
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

// ── Prose element overrides ───────────────────────────────────────────────────

// Theme overrides for prose tags also styled by the user-agent
// [`default_element_rules`]. Appended after them in `all_rules`, so the later
// (theme) rule wins the same-specificity tag cascade on both the terminal and
// the serialized stylesheet: links pick up `Primary`, code spans/blocks a
// `SurfaceContainerHighest` fill with `OnSurface` text.

/// Anchor text in the theme's primary color.
pub fn link_prose() -> Rule {
	Rule::new()
		.with_selector(Selector::tag("a"))
		.with_token(common_props::ForegroundColor,colors::Primary).unwrap()
}

/// Inline `<code>` - filled chip readable against the page surface, with a
/// faint rounded corner and a slim inset so the fill clears the glyphs. The
/// vertical inset rounds to zero terminal rows, so it leaves the line height
/// untouched on both targets.
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
		// .with_token(common_props::BackgroundColor,colors::SurfaceContainerLow).unwrap()
		.with_token(common_props::ElevationProp,geometry::Elevation1).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnSurfaceVariant).unwrap()
		.with_token(common_props::BorderColorProp,colors::Primary).unwrap()
		.with_token(common_props::BorderLeftWidth,geometry::OutlineWidthThick).unwrap()
		.with_token(ShapeProps,geometry::ShapeExtraSmall).unwrap()
		.with_canonical(FontStyle::Italic)
		.with_value(common_props::Padding, Spacing::all(Length::Rem(1.)))
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
		.with_value(common_props::ColumnGapProp, Length::Rem(1.0))
		// padded on web and print; the terminal trims this to a single compact
		// line via `app_bar_terminal`.
		.with_value(common_props::Padding, Spacing {
			top: Length::Rem(0.75),
			bottom: Length::Rem(0.75),
			left: Length::Rem(1.5),
			right: Length::Rem(1.5),
		})
}

/// Terminal app bar - trims the vertical padding so the bar stays a single row,
/// keeping a one-cell left inset so the title clears the edge.
pub fn app_bar_terminal() -> Rule {
	Rule::new()
		.with_media(MediaQuery::Terminal)
		.with_selector(Selector::class(APP_BAR))
		.with_value(common_props::Padding, Spacing {
			left: Length::Rem(1.0),
			..Spacing::DEFAULT
		})
}

/// App bar navigation - a flex row so its links are spaced rather than running
/// together as adjacent inline anchors.
pub fn app_bar_nav() -> Rule {
	Rule::new()
		.with_selector(Selector::class(APP_BAR_NAV))
		.with_value(common_props::DisplayProp, Display::Flex)
		.with_value(common_props::ColumnGapProp, Length::Rem(1.0))
}

/// App bar title link - the brand wordmark. Larger than body type, undecorated,
/// and using the surface foreground rather than the prose-link primary color so
/// it reads as a title rather than a hyperlink.
pub fn app_bar_title() -> Rule {
	Rule::new()
		.with_selector(Selector::class("app-bar-title"))
		.with_token(TypographyProps,typography::TitleLarge).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
		.with_canonical(DecorationLine::DEFAULT)
}

/// Page `<footer>` - mirrors the app bar's elevation with a top divider.
///
/// A flex row so the centered copyright and the right-aligned build info sit on
/// one line, flanked by the growing [`footer_side`] cells. The row wraps, so on
/// a terminal too narrow for one line the build info drops to its own full-width
/// row and word-wraps rather than being squeezed into a sliver and broken
/// mid-word.
pub fn footer() -> Rule {
	Rule::new()
		.with_selector(Selector::tag("footer"))
		.with_token(common_props::BackgroundColor,colors::SurfaceContainer).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnSurfaceVariant).unwrap()
		.with_token(common_props::BorderColorProp,colors::OutlineVariant).unwrap()
		.with_token(common_props::BorderTopWidth,geometry::OutlineWidthThin).unwrap()
		.with_value(common_props::DisplayProp, Display::Flex)
		.with_value(common_props::FlexWrapProp, FlexWrap::Wrap)
		.with_value(common_props::AlignItemsProp, AlignItems::Center)
		.with_value(common_props::ColumnGapProp, Length::Rem(1.0))
		.with_value(common_props::Padding, Spacing {
			left: Length::Rem(1.),
			right: Length::Rem(1.),
			..Spacing::DEFAULT
		})
}

/// Footer side cell - grows to push the copyright to the centre and the build
/// info to the right edge.
pub fn footer_side() -> Rule {
	Rule::new()
		.with_selector(Selector::class(FOOTER_SIDE))
		.with_value(common_props::FlexGrowProp, 1u32)
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

/// Page - full page background using the base surface color, with a comfortable
/// large body type as the default reading size.
pub fn page() -> Rule {
	Rule::new()
		.with_selector(Selector::class(PAGE))
		.with_token(common_props::BackgroundColor,colors::Surface).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
		.with_token(TypographyProps,typography::BodyLarge).unwrap()
}

// ── Form controls ───────────────────────────────────────────────────────────

/// `<form>` layout - a vertical stack whose fields stretch to the form width, so
/// a bare `<form>` of labels and inputs reads as key-over-value rows with a gap
/// between each field. Works on both targets.
pub fn form_layout() -> Rule {
	Rule::new()
		.with_selector(Selector::tag("form"))
		.with_value(common_props::DisplayProp, Display::Flex)
		.with_value(common_props::FlexDirectionProp, Direction::Vertical)
		.with_value(common_props::AlignItemsProp, AlignItems::Stretch)
		.with_value(common_props::RowGapProp, Length::Rem(1.0))
}

/// Field `<label>` - a block sitting just above its input, medium weight so the
/// key reads as a heading for its value.
pub fn label_field() -> Rule {
	Rule::new()
		.with_selector(Selector::tag("label"))
		.with_value(common_props::DisplayProp, Display::Block)
		.with_token(common_props::FontWeightProp,typography::WeightMedium).unwrap()
		.with_value(common_props::MarginProp, Spacing {
			bottom: Length::Rem(0.25),
			..Spacing::DEFAULT
		})
}

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

/// Header cells - medium weight, left aligned, padded, with a solid bottom rule
/// separating the header from the body.
pub fn table_th() -> Rule {
	Rule::new()
		.with_selector(Selector::tag("th"))
		.with_token(common_props::FontWeightProp,typography::WeightMedium).unwrap()
		.with_token(common_props::BorderColorProp,colors::Outline).unwrap()
		.with_token(common_props::BorderBottomWidth,geometry::OutlineWidthThin).unwrap()
		.with_value(common_props::TextAlignProp, TextAlign::Left)
		.with_value(common_props::Padding, Spacing::all(Length::Rem(0.5)))
}

/// Bordered table - draws a full cell grid. It sets only the *inherited* uniform
/// `border-width`; `th`/`td` carry the border *color*, while `thead`/`tbody`/`tr`
/// do not, so the width inherits all the way down but only the cells (which have
/// a color) actually paint, yielding vertical dividers without per-cell rules.
pub fn table_bordered() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TABLE_BORDERED))
		.with_token(common_props::OutlineWidth,geometry::OutlineWidthThin).unwrap()
}

/// Body cells - padded, with a faint divider rule below each row.
pub fn table_td() -> Rule {
	Rule::new()
		.with_selector(Selector::tag("td"))
		.with_token(common_props::BorderColorProp,colors::OutlineVariant).unwrap()
		.with_token(common_props::BorderBottomWidth,geometry::OutlineWidthThin).unwrap()
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
		.with_token(common_props::BackgroundColor,colors::SurfaceContainerLow).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
		.with_token(TypographyProps,typography::BodyLarge).unwrap()
		.with_token(common_props::BorderColorProp,colors::OutlineVariant).unwrap()
		.with_token(common_props::BorderRightWidth,geometry::OutlineWidthThin).unwrap()
		.with_value(common_props::Padding, Spacing {
			right: Length::Rem(1.),
			..Spacing::DEFAULT
		})
}

/// Sidebar link - an undecorated link in the faint surface-variant foreground,
/// lifting to the active highlight via [`sidebar_active`]. Fills the rail width
/// as a padded block so the active highlight reads as a full-width pill; the
/// terminal collapses it back to an inline run via [`sidebar_link_terminal`].
pub fn sidebar_link() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SIDEBAR_LINK))
		.with_token(common_props::ForegroundColor,colors::OnSurfaceVariant).unwrap()
		.with_token(ShapeProps,geometry::ShapeExtraSmall).unwrap()
		.with_canonical(DecorationLine::DEFAULT)
		.with_value(common_props::DisplayProp, Display::Block)
		.with_value(common_props::Padding, Spacing {
			top: Length::Rem(0.25),
			bottom: Length::Rem(0.25),
			left: Length::Rem(0.5),
			right: Length::Rem(0.5),
		})
}

/// Terminal sidebar link - inline with no padding, so a link adds no per-item
/// left inset to the terminal nav tree (the block padding is a web affordance).
pub fn sidebar_link_terminal() -> Rule {
	Rule::new()
		.with_media(MediaQuery::Terminal)
		.with_selector(Selector::class(SIDEBAR_LINK))
		.with_value(common_props::DisplayProp, Display::Inline)
		.with_value(common_props::Padding, Spacing::DEFAULT)
}

/// The current page in the sidebar - primary text on a raised surface, matching
/// the `aria-current="page"` leaf or branch link. An attribute selector, so it
/// works the same on both targets.
pub fn sidebar_active() -> Rule {
	Rule::new()
		.with_selector(Selector::attribute("aria-current", Some("page".into())))
		.with_token(common_props::ForegroundColor,colors::Primary).unwrap()
		.with_token(common_props::BackgroundColor,colors::SurfaceContainerHigh).unwrap()
		.with_token(common_props::FontWeightProp,typography::WeightMedium).unwrap()
		.with_token(ShapeProps,geometry::ShapeExtraSmall).unwrap()
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

// ── Web-only overrides ────────────────────────────────────────────────────────
// `@media screen` rules: the charcell cascade ignores media-gated rules, so
// these refine the web look without touching the terminal, which keeps its
// colored headings and grows to fit its content.

/// On the web, the `.page` body fills at least the viewport height as a flex
/// column, so a short page still pins its footer to the bottom of the screen.
pub fn page_fill_viewport() -> Rule {
	Rule::new()
		.with_media(MediaQuery::Screen)
		.with_selector(Selector::class(PAGE))
		.with_value(common_props::DisplayProp, Display::Flex)
		.with_value(common_props::FlexDirectionProp, Direction::Vertical)
		.with_value(common_props::MinHeight, Length::ViewportHeight(100.))
}

/// On the web, the sidebar + main row grows to fill the space above the footer
/// inside the [`page_fill_viewport`] column.
pub fn container_grow_web() -> Rule {
	Rule::new()
		.with_media(MediaQuery::Screen)
		.with_selector(Selector::class(CONTAINER))
		.with_value(common_props::FlexGrowProp, 1u32)
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
		app_bar_title(),
		app_bar_terminal(),
		footer(),
		footer_side(),
		container(),
		// prose overrides — appended so they win the tag cascade over the
		// user-agent element defaults (later rule wins ties)
		link_prose(),
		code_prose(),
		pre_prose(),
		blockquote_prose(),
		main_content(),
		page(),
		// form controls — state/compound rules first so they win the cascade
		input_focus(),
		select_focus(),
		form_layout(),
		label_field(),
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
		table_bordered(),
		table_th(),
		table_td(),
		// disclosure + sidebar
		details(),
		summary(),
		sidebar(),
		sidebar_link(),
		sidebar_link_terminal(),
		sidebar_active(),
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
		// web-only overrides — gated behind `@media screen`, ignored by charcell
		page_fill_viewport(),
		container_grow_web(),
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
