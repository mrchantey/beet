//! Sidebar and disclosure (`<details>`) classes and their Material Design 3 rules.
#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::*;
use crate::style::material::*;
use beet_core::prelude::Duration;

/// Viewport width (px) at or below which the sidebar collapses behind the
/// [`MENU_BUTTON`] toggle. Shared by the responsive rules here and the
/// `sidebar.js` runtime so the CSS breakpoint and the resize handler agree.
pub const SIDEBAR_BREAKPOINT_PX: u32 = 1024;

// ── Class names ─────────────────────────────────────────────────────────────────
pub const SIDEBAR: ClassName = ClassName::new_static("sidebar");
/// The header toggle that shows/hides the sidebar on narrow screens.
pub const MENU_BUTTON: ClassName = ClassName::new_static("menu-button");
pub const SIDEBAR_LINK: ClassName = ClassName::new_static("sidebar-link");
pub const SIDEBAR_LABEL: ClassName = ClassName::new_static("sidebar-label");
pub const SIDEBAR_GROUP: ClassName = ClassName::new_static("sidebar-group");
/// A `<summary>` row in the sidebar: label on the left, disclosure caret right.
pub const SIDEBAR_SUMMARY: ClassName = ClassName::new_static("sidebar-summary");
/// The disclosure caret on a sidebar group's summary.
pub const SIDEBAR_CARET: ClassName = ClassName::new_static("sidebar-caret");
/// The label/link of a `<summary>` branch row, grown to fill the row so its
/// (active) highlight reaches the right-hand caret.
pub const SIDEBAR_BRANCH: ClassName = ClassName::new_static("sidebar-branch");
/// A nested `<ul>` of sidebar items, with no list block spacing.
pub const SIDEBAR_LIST: ClassName = ClassName::new_static("sidebar-list");
/// A non-root sidebar item, indented one level under its parent group.
pub const SIDEBAR_ITEM: ClassName = ClassName::new_static("sidebar-item");
/// A root (top-level) sidebar item, flush with the rail's left edge.
pub const SIDEBAR_ITEM_ROOT: ClassName =
	ClassName::new_static("sidebar-item-root");

// ── Rules ─────────────────────────────────────────────────────────────────────

/// Sidebar group summary - a flex row on both targets: the label/link grows to
/// fill the row (see [`sidebar_branch`]) and the disclosure caret sits at the
/// right edge. `list-style: none` drops the browser's default left disclosure
/// triangle (replaced by the right-hand caret). The generic `<details>`/
/// `<summary>` block + cursor rules live in `style::elements`.
pub fn sidebar_summary() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SIDEBAR_SUMMARY))
		.with_canonical(ListStyle::None)
		.with_value(common_props::DisplayProp, Display::Flex)
		.with_value(common_props::AlignItemsProp, AlignItems::Center)
}

/// Sidebar branch label/link - grows to fill its `<summary>` row so its padded
/// block (and active highlight) runs full-width up to the right-hand caret,
/// matching how a leaf link fills its row.
pub fn sidebar_branch() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SIDEBAR_BRANCH))
		.with_value(common_props::FlexGrowProp, 1u32)
}

/// Sidebar disclosure caret - faint, sitting at the right edge of its summary,
/// larger than the row text so it reads as a clear affordance. A single
/// down-caret glyph; the web rotates it to point right when the group is
/// collapsed (see [`sidebar_caret_collapsed`]), the transition smoothing the
/// flip. The terminal can't rotate and always shows children, so the static
/// down-caret reads correctly there.
pub fn sidebar_caret() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SIDEBAR_CARET))
		.with_token(common_props::ForegroundColor,colors::OnSurfaceVariant).unwrap()
		.with_value(common_props::FontSize, Length::Rem(1.5))
		.with_value(common_props::TransitionDurationProp, Duration::from_millis(150))
}

/// Web caret rotation - a collapsed `<details>` (no `open` attribute) points its
/// caret right via a 90° rotation. A descendant combinator (`details:not([open])
/// .sidebar-caret`), so it's web-only: the charcell cascade has no ancestor
/// context and the terminal always renders children expanded anyway.
pub fn sidebar_caret_collapsed() -> Rule {
	Rule::new()
		.with_media(MediaQuery::Screen)
		.with_selector(Selector::descendant(
			Selector::AllOf(vec![
				Selector::tag("details"),
				Selector::not(Selector::attribute("open", None)),
			]),
			Selector::class(SIDEBAR_CARET),
		))
		.with_value(common_props::TransformProp, Transform::Rotate(-90.))
}

/// Nested sidebar `<ul>` - drops the prose list's block spacing so the tree's
/// rows sit flush (overriding the `ul` block-gap margin on both targets).
pub fn sidebar_list() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SIDEBAR_LIST))
		.with_value(common_props::MarginProp, Spacing::DEFAULT)
}

/// Sidebar nav container - a left rail divided from the main column by a
/// right border, with padding so its links clear the divider.
pub fn sidebar() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SIDEBAR))
		.with_token(common_props::BackgroundColor,colors::SurfaceContainerLow).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
		.with_token(TypographyProps,typography::TitleMedium).unwrap()
		.with_token(common_props::BorderColorProp,colors::OutlineVariant).unwrap()
		.with_token(common_props::BorderRightWidth,geometry::OutlineWidthThin).unwrap()
		.with_value(common_props::Padding, Spacing {
			right: Length::Rem(1.),
			..Spacing::DEFAULT
		})
}

/// Web sidebar - a comfortable fixed rail width so the nav tree isn't cramped.
/// `min-width` pins the width: the rail is a flex item beside the main column,
/// so without a floor the main content's preferred width shrinks it (a varying
/// amount per page). Screen-gated: the terminal sizes the rail to its content.
pub fn sidebar_web() -> Rule {
	Rule::new()
		.with_media(MediaQuery::Screen)
		.with_selector(Selector::class(SIDEBAR))
		.with_value(common_props::Width, Length::Rem(16.))
		.with_value(common_props::MinWidth, Length::Rem(16.))
		.with_value(common_props::Padding, Spacing {
			left: Length::Rem(0.5),
			right: Length::Rem(1.),
			top: Length::Rem(0.5),
			bottom: Length::Rem(0.5),
		})
}

/// Web sidebar collapse - on screens at or below [`SIDEBAR_BREAKPOINT_PX`] the
/// rail is taken out of flow when `sidebar.js` marks it `aria-hidden="true"`
/// (its default on load below the breakpoint, toggled by the [`MENU_BUTTON`]).
/// Screen-gated and attribute-driven, so the terminal - which never sets the
/// attribute - keeps the rail.
pub fn sidebar_hidden() -> Rule {
	Rule::new()
		.with_media(MediaQuery::MaxWidth(SIDEBAR_BREAKPOINT_PX))
		.with_selector(Selector::AllOf(vec![
			Selector::class(SIDEBAR),
			Selector::attribute("aria-hidden", Some("true".into())),
		]))
		.with_value(common_props::DisplayProp, Display::None)
}

/// Menu button - hidden by default on every target; the wide-screen sidebar is
/// always visible (and the terminal rail too) so the toggle is unnecessary.
/// Ungated so the terminal cascade also hides it; the web reveals it below the
/// breakpoint via [`menu_button_visible`]. A larger glyph than the nav buttons
/// with no horizontal padding, so it reads as a compact icon affordance flush
/// against the title.
pub fn menu_button() -> Rule {
	Rule::new()
		.with_selector(Selector::class(MENU_BUTTON))
		.with_value(common_props::DisplayProp, Display::None)
		.with_value(common_props::FontSize, Length::Rem(2.))
		.with_value(common_props::Padding, Spacing {
			left: Length::Rem(0.),
			right: Length::Rem(0.),
			top: Length::Rem(0.4),
			bottom: Length::Rem(0.4),
		})
}

/// Menu button on narrow screens - shown at or below [`SIDEBAR_BREAKPOINT_PX`],
/// where the sidebar collapses and needs a toggle.
pub fn menu_button_visible() -> Rule {
	Rule::new()
		.with_media(MediaQuery::MaxWidth(SIDEBAR_BREAKPOINT_PX))
		.with_selector(Selector::class(MENU_BUTTON))
		.with_value(common_props::DisplayProp, Display::Flex)
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
		// full-width block so the whole row is the click/hover target (the
		// hit-test resolves the row's rect, not just the painted text) and the
		// hover state layer fills the rail.
		.with_value(common_props::DisplayProp, Display::Block)
		.with_value(common_props::Padding, Spacing {
			top: Length::Rem(0.25),
			bottom: Length::Rem(0.25),
			left: Length::Rem(0.5),
			right: Length::Rem(0.5),
		})
}

/// Terminal sidebar row - drops the web block padding from links and labels so a
/// row adds no per-item left inset to the terminal nav tree (the padding is a web
/// affordance). The row stays a full-width `display: block`, so the active
/// highlight fills the rail rather than hugging the text.
pub fn sidebar_link_terminal() -> Rule {
	Rule::new()
		.with_media(MediaQuery::Terminal)
		.with_selector(Selector::class(SIDEBAR_LINK).merge_any(Selector::class(SIDEBAR_LABEL)))
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
		.with_selector(Selector::class(SIDEBAR_ITEM))
		.with_value(common_props::Padding, Spacing {
			left: Length::Rem(1.),
			..Spacing::DEFAULT
		})
}

/// Sidebar group label - faint, for non-navigable headers. Carries the same
/// padded block as [`sidebar_link`] so an anchorless row (a group with no route)
/// lines its text up with the link rows beside it rather than sitting a padding
/// step to the left. The terminal strips the padding via [`sidebar_link_terminal`].
pub fn sidebar_label() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SIDEBAR_LABEL))
		.with_token(common_props::ForegroundColor,colors::OnSurfaceVariant).unwrap()
		.with_token(common_props::FontWeightProp,typography::WeightMedium).unwrap()
		.with_value(common_props::DisplayProp, Display::Block)
		.with_value(common_props::Padding, Spacing {
			top: Length::Rem(0.25),
			bottom: Length::Rem(0.25),
			left: Length::Rem(0.5),
			right: Length::Rem(0.5),
		})
}
