//! The `.toast` class and its Material Design 3 snackbar rule.
#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::*;
use crate::style::material::*;

// ── Class names ─────────────────────────────────────────────────────────────────
pub const TOAST: ClassName = ClassName::new_static("toast");

// ── Rules ─────────────────────────────────────────────────────────────────────

/// A transient overlay box (the [`Toast`](crate::prelude::Toast) widget): fixed
/// to the viewport's bottom-right corner and lifted above all page content, on
/// the inverse-surface palette MD3 reserves for snackbars. The high z-index
/// clears the `.select-dropdown` overlay (1000); `Position::Fixed` anchors it to
/// the buffer viewport so it floats over whatever scrolls beneath.
pub fn toast() -> Rule {
	Rule::new()
		.with_selector(Selector::class(TOAST))
		.with_value(common_props::PositionProp, Position::Fixed)
		.with_value(common_props::InsetBottom, Length::Rem(1.))
		.with_value(common_props::InsetRight, Length::Rem(1.))
		.with_value(common_props::ZIndexProp, 1100)
		.with_token(common_props::BackgroundColor,colors::InverseSurface).unwrap()
		.with_token(common_props::ForegroundColor,colors::InverseOnSurface).unwrap()
		.with_token(ShapeProps,geometry::ShapeExtraSmall).unwrap()
		.with_token(TypographyProps,typography::BodyMedium).unwrap()
		.with_value(common_props::Padding, Spacing {
			top: Length::Rem(0.5),
			bottom: Length::Rem(0.5),
			left: Length::Rem(1.),
			right: Length::Rem(1.),
		})
}
