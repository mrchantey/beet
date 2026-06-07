//! Document-shell layout classes and their Material Design 3 rules.
#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::*;
use crate::style::material::*;

// ── Class names ─────────────────────────────────────────────────────────────────
pub const APP_BAR: ClassName = ClassName::new_static("app-bar");
pub const APP_BAR_SCROLLED: ClassName = ClassName::new_static("app-bar-scrolled");
pub const APP_BAR_NAV: ClassName = ClassName::new_static("app-bar-nav");
pub const CONTAINER: ClassName = ClassName::new_static("container");
pub const PAGE: ClassName = ClassName::new_static("page");
/// A footer side cell that grows to flank the centered copyright.
pub const FOOTER_SIDE: ClassName = ClassName::new_static("footer-side");

// ── Rules ─────────────────────────────────────────────────────────────────────

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
