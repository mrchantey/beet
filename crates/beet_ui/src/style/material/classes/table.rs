//! Table classes and their Material Design 3 rules.
#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::*;
use crate::style::material::*;

// ── Class names ─────────────────────────────────────────────────────────────────
pub const TABLE: ClassName = ClassName::new_static("table");
/// A table with internal vertical column dividers, in addition to the default
/// horizontal row rules.
pub const TABLE_VERTICAL_BORDERS: ClassName =
	ClassName::new_static("table-vertical-borders");

// ── Rules ─────────────────────────────────────────────────────────────────────

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

/// Body cells - padded, with a faint divider rule below each row.
pub fn table_td() -> Rule {
	Rule::new()
		.with_selector(Selector::tag("td"))
		.with_token(common_props::BorderColorProp,colors::OutlineVariant).unwrap()
		.with_token(common_props::BorderBottomWidth,geometry::OutlineWidthThin).unwrap()
		.with_value(common_props::Padding, Spacing::all(Length::Rem(0.5)))
}
