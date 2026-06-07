//! Generic utility classes (color, visibility, print, motion, accessibility) and
//! their Material Design 3 rules.
#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::*;
use crate::style::material::*;
use beet_core::prelude::Duration;

// ── Class names ─────────────────────────────────────────────────────────────────
pub const COLOR_PRIMARY: ClassName = ClassName::new_static("color-primary");
pub const HIDDEN: ClassName = ClassName::new_static("hidden");
// Print utilities, styled by `@media print` rules.
pub const PRINT_HIDDEN: ClassName = ClassName::new_static("print-hidden");
pub const PAGE_BREAK: ClassName = ClassName::new_static("page-break");

// ── Color utility classes ─────────────────────────────────────────────────────

/// Primary color scheme - primary background with on-primary text.
pub fn color_primary() -> Rule {
	Rule::new()
		.with_selector(Selector::class(COLOR_PRIMARY))
		.with_token(ColorRoleProps,colors::PrimaryRole).unwrap()
}

// ── Visibility / print utilities ──────────────────────────────────────────────

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

// ── Interaction ─────────────────────────────────────────────────────────────

/// The set of interactive elements that share the hover affordance: every
/// `<button>` (and `.btn`-styled link) plus any `<a>`, so buttons, prose links,
/// and sidebar anchors all respond to hover identically.
fn interactive() -> Selector {
	Selector::AnyOf(vec![Selector::tag("button"), Selector::tag("a")])
}

/// `:hover` over an interactive element: the same selectors, gated on the
/// [`Hovered`](ElementState::Hovered) state.
fn interactive_hover() -> Selector {
	Selector::AnyOf(vec![
		Selector::AllOf(vec![Selector::tag("button"), Selector::state(ElementState::Hovered)]),
		Selector::AllOf(vec![Selector::tag("a"), Selector::state(ElementState::Hovered)]),
	])
}

/// Eases the [hover dim](hover_dim) in and out on every interactive element.
/// With no explicit `transition-property` the browser animates all changes, so
/// the same short duration also smooths the active state and focus ring.
pub fn interactive_transition() -> Rule {
	Rule::new()
		.with_selector(interactive())
		.with_token(common_props::TransitionDurationProp,motion::Short4).unwrap()
}

/// Hover affordance - a slight whole-element dim on `button:hover`/`a:hover`,
/// animated by [`interactive_transition`]. Opacity is uniform across element
/// types and the one effect that reads the same on a filled button, a text link,
/// and a sidebar row.
pub fn hover_dim() -> Rule {
	Rule::new()
		.with_selector(interactive_hover())
		.with_value(common_props::OpacityProp, 0.8_f32)
}

// ── Accessibility ─────────────────────────────────────────────────────────────

/// Disabled elements - faint foreground (`:disabled`).
pub fn disabled_state() -> Rule {
	Rule::new()
		.with_selector(Selector::state(ElementState::Disabled))
		.with_token(common_props::ForegroundColor,colors::OnSurfaceVariant).unwrap()
}
