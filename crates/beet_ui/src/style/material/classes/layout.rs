//! Document-layout classes and their Material Design 3 rules.
#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::*;
use crate::style::material::*;

// ── Class names ─────────────────────────────────────────────────────────────────
pub const APP_BAR: ClassName = ClassName::new_static("app-bar");
pub const APP_BAR_SCROLLED: ClassName = ClassName::new_static("app-bar-scrolled");
pub const APP_BAR_NAV: ClassName = ClassName::new_static("app-bar-nav");
/// The app bar's leading cluster: an optional control (eg the sidebar menu
/// button) sitting immediately left of the title.
pub const APP_BAR_LEADING: ClassName = ClassName::new_static("app-bar-leading");
pub const CONTAINER: ClassName = ClassName::new_static("container");
pub const PAGE: ClassName = ClassName::new_static("page");
/// A footer side cell that grows to flank the centered copyright.
pub const FOOTER_SIDE: ClassName = ClassName::new_static("footer-side");
/// A 12-column grid container with square row tracks (see [`grid`]).
pub const GRID: ClassName = ClassName::new_static("grid");

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

/// Web app bar - wraps its leading cluster and nav onto separate rows once they
/// no longer fit on one line, so a very narrow screen (eg a 320px phone) drops
/// the Docs/Blog/GitHub nav below the wordmark rather than overflowing the page
/// width. Inert while the bar fits (a single-row `space-between` as before).
/// Screen-gated: the terminal keeps its deliberately single-row app bar (see
/// [`app_bar_terminal`]).
pub fn app_bar_web() -> Rule {
	Rule::new()
		.with_media(MediaQuery::Screen)
		.with_selector(Selector::class(APP_BAR))
		.with_value(common_props::FlexWrapProp, FlexWrap::Wrap)
}

/// App bar navigation - a flex row so its links are spaced rather than running
/// together as adjacent inline anchors. Centers on the cross axis so links of
/// differing heights (a bordered button beside a text link) sit on one line.
pub fn app_bar_nav() -> Rule {
	Rule::new()
		.with_selector(Selector::class(APP_BAR_NAV))
		.with_value(common_props::DisplayProp, Display::Flex)
		.with_value(common_props::AlignItemsProp, AlignItems::Center)
		.with_value(common_props::ColumnGapProp, Length::Rem(1.0))
}

/// Web app bar navigation - drops the column gap, since the nav buttons carry
/// their own horizontal padding on the web. Screen-gated: the terminal keeps the
/// [`app_bar_nav`] gap, where the links render as bare inline runs that would
/// otherwise collide.
pub fn app_bar_nav_web() -> Rule {
	Rule::new()
		.with_media(MediaQuery::Screen)
		.with_selector(Selector::class(APP_BAR_NAV))
		.with_value(common_props::ColumnGapProp, Length::Rem(0.))
}

/// App bar nav link - a step up from the default button label ([`LabelLarge`],
/// 0.875rem) to [`FontSizeBodyLarge`](typography::FontSizeBodyLarge) (1rem) so the
/// header's Docs/Blog/GitHub read comfortably beside the wordmark. Higher
/// specificity than [`button_base`](super::button_base) (`.app-bar-nav .btn` vs
/// `.btn`), so it overrides only the font-size and keeps the button's label
/// weight, spacing, and line box - the line box is unchanged, so the bar's height
/// is too.
pub fn app_bar_nav_link() -> Rule {
	Rule::new()
		.with_selector(Selector::descendant(
			Selector::class(APP_BAR_NAV),
			Selector::class(super::BTN),
		))
		.with_token(common_props::FontSize, typography::FontSizeBodyLarge).unwrap()
}

/// App bar leading cluster - a flex row so a leading control (the menu button)
/// sits beside the title with a comfortable gap, the pair held to the left while
/// the nav stays right via the bar's `space-between`.
pub fn app_bar_leading() -> Rule {
	Rule::new()
		.with_selector(Selector::class(APP_BAR_LEADING))
		.with_value(common_props::DisplayProp, Display::Flex)
		.with_value(common_props::AlignItemsProp, AlignItems::Center)
		.with_value(common_props::ColumnGapProp, Length::Rem(1.0))
}

/// App bar title link - the brand wordmark. Larger than body type, undecorated,
/// and using the surface foreground rather than the prose-link primary color so
/// it reads as a title rather than a hyperlink.
pub fn app_bar_title() -> Rule {
	Rule::new()
		.with_selector(Selector::class("app-bar-title"))
		.with_token(TypographyProps,typography::TitleLarge).unwrap()
		// the longhand `font-size` mirrors the composite so the terminal scales
		// the brand to fullwidth like other title-large text (the charcell
		// renderer scales by `font-size`, not the composite token).
		.with_token(common_props::FontSize,typography::FontSizeTitleLarge).unwrap()
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

/// Grid container - the default 12 columns of square tracks with a one-cell
/// gap. Adjust per usage with [`common_props::GridTemplateColumnsProp`] /
/// [`common_props::GridAutoRowsProp`] rules.
pub fn grid() -> Rule {
	Rule::new()
		.with_selector(Selector::class(GRID))
		.with_value(common_props::DisplayProp, Display::Grid)
		.with_value(common_props::ColumnGapProp, Length::Rem(1.0))
		.with_value(common_props::RowGapProp, Length::Rem(1.0))
}

/// The readable measure the main content column is capped at.
const MAIN_MEASURE_REM: f32 = 70.;

/// Main content column: a centred flex column that grows to fill the space
/// beside the sidebar. Paired with [`main_content_measure`], which caps each
/// child at [`MAIN_MEASURE_REM`], so page content (the index included) reads as
/// a centred column on both the web and the terminal.
///
/// `min-width: 0` lets the column shrink below its intrinsic content width. As a
/// flex item in [`container`], `<main>` would otherwise default to `min-width:
/// auto` (`min-content`), so a wide unbreakable child - a long code line in a
/// `<pre>`, a fixed-size embed - would force the column past the viewport on a
/// narrow screen instead of the child scrolling/scaling within it. A no-op on
/// the terminal, whose charcell box model applies no `min-content` floor.
pub fn main_content() -> Rule {
	Rule::new()
		.with_selector(Selector::tag("main"))
		.with_value(common_props::FlexGrowProp, 1u32)
		.with_value(common_props::MinWidth, Length::Px(0.))
		.with_value(common_props::DisplayProp, Display::Flex)
		.with_value(common_props::FlexDirectionProp, Direction::Vertical)
		.with_value(common_props::AlignItemsProp, AlignItems::Center)
		.with_value(common_props::Padding, Spacing::all(Length::Rem(1.)))
}

/// Cap each direct child of `<main>` at [`MAIN_MEASURE_REM`] so page content
/// reads as a centred column (see [`main_content`]). A direct-child combinator
/// so only top-level blocks are capped, not nested inline content; the charcell
/// cascade honours it too, so the cap applies on the terminal as well.
pub fn main_content_measure() -> Rule {
	Rule::new()
		.with_selector(Selector::child(Selector::tag("main"), Selector::Any))
		.with_value(common_props::Width, Length::Percent(100.))
		.with_value(common_props::MaxWidth, Length::Rem(MAIN_MEASURE_REM))
}

/// Page - the full page background and foreground, with a comfortable large body
/// type as the default reading size.
///
/// Points at the conservative `Background` role (the app base tone) rather than a
/// `Surface`, so the page paints the same neutral base on both the web and the
/// terminal — a card or app bar layered on top is what reads as a distinct
/// surface.
pub fn page() -> Rule {
	Rule::new()
		.with_selector(Selector::class(PAGE))
		.with_token(common_props::BackgroundColor,colors::Background).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnBackground).unwrap()
		.with_token(TypographyProps,typography::BodyLarge).unwrap()
}

// ── Web-only overrides ────────────────────────────────────────────────────────
// `@media screen` rules: the charcell cascade ignores media-gated rules, so
// these refine the web look without touching the terminal, which keeps its
// colored headings and grows to fit its content.

/// The `.page` body fills at least the viewport height as a flex column, so a
/// short page still pins its footer to the bottom — of the screen on the web, and
/// of the terminal window in charcell (the cell viewport resolves `100vh`).
/// Ungated so both targets fill the window; the terminal no longer shrinks to its
/// content.
pub fn page_fill_viewport() -> Rule {
	Rule::new()
		.with_selector(Selector::class(PAGE))
		.with_value(common_props::DisplayProp, Display::Flex)
		.with_value(common_props::FlexDirectionProp, Direction::Vertical)
		// stretch the header/content/footer to the full width (the web's flex
		// default; charcell's flex defaults to `start`, so a column would otherwise
		// shrink each row to its content and the app-bar divider wouldn't span).
		.with_value(common_props::AlignItemsProp, AlignItems::Stretch)
		.with_value(common_props::MinHeight, Length::ViewportHeight(100.))
}

/// The sidebar + main row grows to fill the space above the footer inside the
/// [`page_fill_viewport`] column, on both targets.
pub fn container_grow() -> Rule {
	Rule::new()
		.with_selector(Selector::class(CONTAINER))
		.with_value(common_props::FlexGrowProp, 1u32)
}
