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

/// The set of interactive elements that share the hover affordance on every
/// target: every `<button>` (and `.btn`-styled link) and any `<a>`, so buttons,
/// prose links, and sidebar anchors all respond to hover identically.
fn interactive() -> Selector {
	Selector::AnyOf(vec![
		Selector::tag("button"),
		Selector::tag("a"),
	])
}

/// The terminal's link fallbacks: an `<img>`'s alt placeholder and an
/// `<iframe>`'s collapsed title render as links there, so they share the hover
/// affordance. Rules on this selector must be terminal-gated: on the web these
/// are a real image/frame, never hoverable.
fn interactive_fallback() -> Selector {
	Selector::AnyOf(vec![
		Selector::tag("img"),
		Selector::tag("iframe"),
	])
}

/// Gates a selector on the [`Hovered`](ElementState::Hovered) state.
fn hovered(selector: Selector) -> Selector {
	Selector::AllOf(vec![selector, Selector::state(ElementState::Hovered)])
}

/// `:hover` over an interactive element: the [`interactive`] tags, each gated
/// on the [`Hovered`](ElementState::Hovered) state.
fn interactive_hover() -> Selector {
	Selector::AnyOf(vec![
		hovered(Selector::tag("button")),
		hovered(Selector::tag("a")),
	])
}

/// `:hover` over a terminal link fallback: the [`interactive_fallback`] tags,
/// each gated on the [`Hovered`](ElementState::Hovered) state.
fn interactive_fallback_hover() -> Selector {
	Selector::AnyOf(vec![
		hovered(Selector::tag("img")),
		hovered(Selector::tag("iframe")),
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

/// Terminal companion of [`interactive_transition`] for the `<img>`/`<iframe>`
/// link fallbacks.
pub fn interactive_fallback_transition() -> Rule {
	Rule::new()
		.with_selector(interactive_fallback())
		.with_media(MediaQuery::Terminal)
		.with_token(common_props::TransitionDurationProp,motion::Short4).unwrap()
}

/// Hover affordance - a slight whole-element dim on `button:hover`/`a:hover`,
/// animated by [`interactive_transition`]. On a filled element (or any dark
/// scheme) this reads as a subtle darkening; on a container-less element in a
/// light scheme it is invisible, so [`hover_state_layer`] adds a fill there.
pub fn hover_dim() -> Rule {
	Rule::new()
		.with_selector(interactive_hover())
		.with_value(common_props::OpacityProp, 0.8_f32)
}

/// Terminal companion of [`hover_dim`] for the `<img>`/`<iframe>` link
/// fallbacks, so a hovered alt/title placeholder dims like an `<a>`.
pub fn hover_dim_fallback() -> Rule {
	Rule::new()
		.with_selector(interactive_fallback_hover())
		.with_media(MediaQuery::Terminal)
		.with_value(common_props::OpacityProp, 0.8_f32)
}

/// The container-less interactives that need a hover *fill* (the opacity dim is
/// invisible on them in a light scheme): text/outline buttons, links, sidebar
/// rows, and disclosure summaries/carets — the latter so a `<details>` arrow
/// hovers like a text button.
fn container_less_hover() -> Selector {
	Selector::AnyOf(vec![
		hovered(Selector::class(super::BTN_TEXT)),
		hovered(Selector::class(super::BTN_OUTLINED)),
		hovered(Selector::class(super::SIDEBAR_LINK)),
		hovered(Selector::class(super::SIDEBAR_CARET)),
		hovered(Selector::tag("summary")),
	])
}

/// Hover state layer for container-less interactives - a faint surface fill on
/// hover, redirecting to the [`HoverSurface`](colors::HoverSurface) token.
///
/// That token is set only under `.light-scheme` (see [`hover_surface_light`]),
/// so in a dark scheme it fails to resolve and the background stays unset — the
/// hover there is the [`hover_dim`] text darkening alone, matching how the dark
/// theme is meant to read. (A backgroundless hovered row stays its own hit target
/// regardless: the hit-test resolves the row's box, not its fill.)
pub fn hover_state_layer() -> Rule {
	Rule::new()
		.with_selector(container_less_hover())
		.with_token(common_props::BackgroundColor, colors::HoverSurface).unwrap()
}

/// Binds the light scheme's [`HoverSurface`](colors::HoverSurface) to a raised
/// surface tone. A standalone `.light-scheme` rule (not baked into the `:root`
/// defaults) so the token is genuinely absent in the dark scheme.
pub fn hover_surface_light() -> Rule {
	Rule::new()
		.with_selector(Selector::class(super::LIGHT_SCHEME))
		.with_token(colors::HoverSurface, colors::SurfaceContainerHigh).unwrap()
}

// ── Accessibility ─────────────────────────────────────────────────────────────

/// Disabled elements - faint foreground (`:disabled`).
pub fn disabled_state() -> Rule {
	Rule::new()
		.with_selector(Selector::state(ElementState::Disabled))
		.with_token(common_props::ForegroundColor,colors::OnSurfaceVariant).unwrap()
}
