//! Button classes and their Material Design 3 rules.
#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::*;
use crate::style::material::*;

// ── Class names ─────────────────────────────────────────────────────────────────
pub const BTN: ClassName = ClassName::new_static("btn");
pub const BTN_FILLED: ClassName = ClassName::new_static("btn-filled");
pub const BTN_OUTLINED: ClassName = ClassName::new_static("btn-outlined");
pub const BTN_TEXT: ClassName = ClassName::new_static("btn-text");
pub const BTN_TONAL: ClassName = ClassName::new_static("btn-tonal");
pub const BTN_ELEVATED: ClassName = ClassName::new_static("btn-elevated");
pub const BTN_SECONDARY: ClassName = ClassName::new_static("btn-secondary");
pub const BTN_TERTIARY: ClassName = ClassName::new_static("btn-tertiary");
pub const BTN_ERROR: ClassName = ClassName::new_static("btn-error");
pub const BTN_ICON: ClassName = ClassName::new_static("btn-icon");

// ── Rules ─────────────────────────────────────────────────────────────────────

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
/// radius. The animated [hover dim](crate::prelude::hover_dim) and its easing
/// transition are shared with links via [`interactive_transition`].
pub fn button_base() -> Rule {
	Rule::new()
		.with_selector(Selector::tag("button").merge_any(Selector::class(BTN)))
		.with_token(TypographyProps,typography::LabelLarge).unwrap()
		.with_token(ShapeProps,geometry::ShapeSmall).unwrap()
		.with_value(common_props::Padding, button_padding())
		.with_value(common_props::CursorProp, Cursor::Pointer)
		.with_canonical(DecorationLine::DEFAULT)
}

/// A contained (filled-style) button: a solid `color` surface with `on_color`
/// text and the given `elevation`.
///
/// Carries no border: the fill alone reads as a solid surface, so a contained
/// button stays a single line of text in the terminal rather than a three-row
/// box. It lines up with the taller [outlined](button_outlined) button through
/// cross-axis centering of its flex row, not by matching its box size.
fn button_contained(
	class: ClassName,
	color: impl Into<Token>,
	on_color: impl Into<Token>,
	elevation: impl Into<Token>,
) -> Rule {
	Rule::new()
		.with_selector(Selector::class(class))
		.with_token(common_props::BackgroundColor, color).unwrap()
		.with_token(common_props::ForegroundColor, on_color).unwrap()
		.with_token(common_props::ElevationProp, elevation).unwrap()
}

/// Filled button - the primary action button with high emphasis.
pub fn button_filled() -> Rule {
	button_contained(BTN_FILLED, colors::Primary, colors::OnPrimary, geometry::Elevation0)
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

/// Text button - lowest emphasis, no container, regular surface-foreground text
/// (not the primary accent, so it reads as a plain control rather than a link).
pub fn button_text() -> Rule {
	Rule::new()
		.with_selector(Selector::class(BTN_TEXT))
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
}

/// Tonal button - medium emphasis with secondary container color.
pub fn button_tonal() -> Rule {
	button_contained(BTN_TONAL, colors::SecondaryContainer, colors::OnSecondaryContainer, geometry::Elevation0)
}

/// Elevated button - medium emphasis with shadow elevation, regular foreground.
pub fn button_elevated() -> Rule {
	button_contained(BTN_ELEVATED, colors::Surface, colors::OnSurface, geometry::Elevation1)
}

/// Secondary filled button - medium emphasis using the secondary color.
pub fn button_secondary() -> Rule {
	button_contained(BTN_SECONDARY, colors::Secondary, colors::OnSecondary, geometry::Elevation0)
}

/// Tertiary filled button - medium emphasis using the tertiary color.
pub fn button_tertiary() -> Rule {
	button_contained(BTN_TERTIARY, colors::Tertiary, colors::OnTertiary, geometry::Elevation0)
}

/// Error button - destructive action using the error color.
pub fn button_error() -> Rule {
	button_contained(BTN_ERROR, colors::Error, colors::OnError, geometry::Elevation0)
}

/// Icon button - circular, container-less button sized for a single glyph.
pub fn button_icon() -> Rule {
	Rule::new()
		.with_selector(Selector::class(BTN_ICON))
		.with_token(common_props::ForegroundColor,colors::OnSurfaceVariant).unwrap()
		.with_token(ShapeProps,geometry::ShapeFull).unwrap()
}
