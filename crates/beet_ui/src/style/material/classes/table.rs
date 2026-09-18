//! Table classes and their Material Design 3 rules.
#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::*;
use crate::style::material::*;

// ── Class names ─────────────────────────────────────────────────────────────────
pub const TABLE: ClassName = ClassName::new_static("table");
/// The block every table sits in: on the web it scrolls horizontally when the
/// columns outgrow the column, the way a `<pre>` does, instead of pushing the
/// page wider than a phone screen. A `<table>` itself cannot scroll (`overflow`
/// is ignored on a table box) and `display: block` on it would drop the
/// full-width look, so the scroll lives on this wrapper.
pub const TABLE_SCROLL: ClassName = ClassName::new_static("table-scroll");
/// A table with internal vertical column dividers, in addition to the default
/// horizontal row rules.
pub const TABLE_VERTICAL_BORDERS: ClassName =
	ClassName::new_static("table-vertical-borders");

// ── Rules ─────────────────────────────────────────────────────────────────────

/// The [`TABLE_SCROLL`] wrapper: a full-width block with the block gap below
/// (so the next block clears the table rather than butting against its last
/// row) that scrolls horizontally rather than growing past its column. The gap
/// sits here, not on the table, so the web's scrollbar hugs the last row. The
/// terminal scales the columns down to fit instead, so it never overflows.
pub fn table_scroll() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TABLE_SCROLL))
		.with_value(common_props::DisplayProp, Display::Block)
		.with_value(common_props::MaxWidth, Length::Percent(100.))
		.with_value(common_props::OverflowXProp, Overflow::Auto)
		.with_value(common_props::MarginProp, Spacing {
			bottom: Length::Rem(1.),
			..Spacing::DEFAULT
		})
}

/// Table - full-width and surface foreground, sitting in a [`table_scroll`]
/// wrapper that carries its block gap.
///
/// Matches both the `<table>` tag and the `.table` class so a markdown table
/// (which carries no class) gets the same look as the [`Table`](crate::prelude::Table)
/// widget.
pub fn table() -> Rule {
	Rule::new()
		.with_selector(Selector::tag("table").merge_any(Selector::class(TABLE)))
		.with_value(common_props::DisplayProp, Display::Table)
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
		.with_token(TypographyProps,typography::BodyMedium).unwrap()
		.with_value(common_props::Width, Length::Percent(100.))
}

/// The marker rule for [`TABLE_VERTICAL_BORDERS`]: the dividers themselves are
/// drawn per target (an adjacent-sibling rule in `browser_overrides.css` on the
/// web, the charcell decorate system on the terminal), so this declares nothing
/// — it registers the class so the render diagnostics know it is real.
pub fn table_vertical_borders() -> Rule {
	Rule::new().with_selector(Selector::class(TABLE_VERTICAL_BORDERS))
}

/// Header cells - medium weight, left aligned, padded, with a solid bottom rule
/// separating the header from the body.
pub fn table_th() -> Rule {
	Rule::new()
		.with_selector(Selector::tag("th"))
		.with_value(common_props::DisplayProp, Display::TableCell)
		.with_token(common_props::FontWeightProp,typography::WeightMedium).unwrap()
		.with_token(common_props::BorderColorProp,colors::Outline).unwrap()
		.with_token(common_props::BorderBottomWidth,geometry::OutlineWidthThin).unwrap()
		.with_value(common_props::TextAlignProp, TextAlign::Left)
		.with_value(common_props::Padding, Spacing::all(Length::Rem(0.5)))
}

/// Body cells - padded, with a faint divider rule below each row.
pub fn table_td() -> Rule {
	Rule::new()
		.with_selector(Selector::tag("td"))
		.with_value(common_props::DisplayProp, Display::TableCell)
		.with_token(common_props::BorderColorProp,colors::OutlineVariant).unwrap()
		.with_token(common_props::BorderBottomWidth,geometry::OutlineWidthThin).unwrap()
		.with_value(common_props::Padding, Spacing::all(Length::Rem(0.5)))
}
