//! `RenderConsole` widget — a scrollable on-page console mirroring
//! `console.log/info/warn/error` into a styled panel.
//!
//! Web target only. Dropped into a page (eg beside a `<Wasm>` loader), it captures
//! a headless wasm program's output: beet's `log` macros route to `console.*` on
//! wasm, so `info!`/`warn!`/`error!` surface here for free. The panel and per-line
//! level colors are styled by the [`console_rules`] in this module, registered into
//! the global rule set at build and emitted into the page by the `<Stylesheet>` in
//! its `<head>`.
#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::*;
use crate::style::material::*;
use beet_core::prelude::*;
// `Display`/`Overflow` also live in bevy's prelude (glob'd via `beet_core` under the
// `examples`/ml/spatial feature set); pin them to the style tokens used below.
use crate::style::Display;
use crate::style::Overflow;

/// Emits the `#beet-console` panel and the `<script>` that wraps `console.*` to
/// append a styled line per call (see `render_console.js`). The panel is the
/// single root; the inline `<script>` runs once its ancestor element is parsed,
/// so the wrap captures every later log.
///
/// The `id` is the script's handle (`getElementById("beet-console")`) and the
/// `beet-console-panel` class carries its styling; the per-line classes the
/// script sets (`beet-console-error`/`warn`/`info`) are styled by the
/// [`console_rules`] in this module.
#[template]
pub fn RenderConsole() -> impl Bundle {
	rsx! {
		<div id="beet-console" {Classes::new([CONSOLE_PANEL])}>
			<script>{include_str!("./render_console.js")}</script>
		</div>
	}
}

// ── Class names ─────────────────────────────────────────────────────────────────
//
// `pub(crate)`, not `pub`: these are an implementation detail of the widget and
// its rules, so they stay out of the prelude glob. Within the crate they are
// reached prefixed, eg `render_console::CONSOLE_INFO`. The line-level classes
// ([`CONSOLE_ERROR`]/[`CONSOLE_WARN`]/[`CONSOLE_INFO`]) are set by
// `render_console.js` on each captured log line, so their selector strings are
// the contract with that script and must not drift.

/// The scrollable monospace panel captured console output appends to.
pub(crate) const CONSOLE_PANEL: ClassName = ClassName::new_static("beet-console-panel");
/// A single captured log line; one per `console.*` call.
pub(crate) const CONSOLE_LINE: ClassName = ClassName::new_static("beet-console-line");
/// An `error`-level line, colored with the theme's error role.
pub(crate) const CONSOLE_ERROR: ClassName = ClassName::new_static("beet-console-error");
/// A `warn`-level line, colored with the theme's tertiary role.
pub(crate) const CONSOLE_WARN: ClassName = ClassName::new_static("beet-console-warn");
/// An `info`-level line, colored with the theme's primary role.
pub(crate) const CONSOLE_INFO: ClassName = ClassName::new_static("beet-console-info");

// ── Rules ─────────────────────────────────────────────────────────────────────

/// The console panel + per-level line color rules, in cascade order.
///
/// Registered into the global [`RuleSet`] by `widget_plugin` at build, so
/// `<Stylesheet>` emits them with the material color roles resolved (the line
/// classes are set by `render_console.js`).
pub(crate) fn console_rules() -> Vec<Rule> {
	vec![
		console_panel(),
		console_line(),
		console_error(),
		console_warn(),
		console_info(),
	]
}

/// The console panel: a monospace, scrollable surface that wraps long lines yet
/// preserves their own breaks. Colors lean on the material surface roles, so it
/// recolors with the theme.
fn console_panel() -> Rule {
	Rule::new()
		.with_selector(Selector::class(CONSOLE_PANEL))
		.with_token(common_props::FontFamilyProp, typography::TypefaceMono).unwrap()
		.with_token(common_props::BackgroundColor, colors::SurfaceContainer).unwrap()
		.with_token(common_props::ForegroundColor, colors::OnSurface).unwrap()
		.with_token(ShapeProps, geometry::ShapeSmall).unwrap()
		.with_value(common_props::FontSize, Length::Rem(0.85))
		.with_value(common_props::LineHeight, Length::Rem(1.5))
		.with_value(common_props::Padding, Spacing::all(Length::Rem(1.)))
		.with_value(common_props::MaxHeight, Length::ViewportHeight(70.))
		.with_value(common_props::OverflowYProp, Overflow::Auto)
		.with_canonical(WhiteSpace::PreWrap)
		.with_canonical(WordBreak::BreakWord)
}

/// A captured line - a block so each `console.*` call sits on its own row.
fn console_line() -> Rule {
	Rule::new()
		.with_selector(Selector::class(CONSOLE_LINE))
		.with_canonical(Display::Block)
}

/// `error`-level line color.
fn console_error() -> Rule {
	console_level(CONSOLE_ERROR, colors::Error)
}

/// `warn`-level line color.
fn console_warn() -> Rule {
	console_level(CONSOLE_WARN, colors::Tertiary)
}

/// `info`-level line color.
fn console_info() -> Rule {
	console_level(CONSOLE_INFO, colors::Primary)
}

/// A per-level line color rule, pointing `class`'s foreground at a color role.
fn console_level(class: ClassName, color: impl Into<Token>) -> Rule {
	Rule::new()
		.with_selector(Selector::class(class))
		.with_token(common_props::ForegroundColor, color).unwrap()
}

#[cfg(test)]
mod test {
	use super::*;

	/// End-to-end: `widget_plugin` (pulled in transitively by `StylePlugin` via
	/// `BsxDefaultsPlugin`) registers the console rules into the global rule set at
	/// build, so a `<Stylesheet>` emits them with the material color roles resolved
	/// - even though they no longer live in `classes::all_rules`.
	#[beet_core::test]
	fn stylesheet_emits_console_css() {
		let mut world =
			(TemplatePlugin, MaterialStylePlugin, StylePlugin).into_world();
		let root = world.spawn_template(rsx! { <Stylesheet/> }).unwrap().id();
		world.with_state::<ElementQuery, _>(|query| {
			query
				.iter_descendant_values(root)
				.filter_map(|value| value.as_str().ok())
				.collect::<String>()
				.as_str()
				.xpect_contains(".beet-console-error")
				.xpect_contains("var(--material-colors-error)");
		});
	}

	/// The console panel + level rules serialize to the expected CSS, exercising
	/// the `font-family`/`white-space: pre-wrap`/`word-break` props added for it.
	#[beet_core::test]
	fn console_rules_css() {
		let css = CssBuilder::default()
			.with_format_variables(FormatVariables::short())
			.build(
				&default_token_map().with_extend(common_props::token_map()),
				&RuleSet::new(Rule::new()).with_rules(console_rules()),
			)
			.unwrap();
		css.as_str()
			.xpect_contains(".beet-console-panel")
			.xpect_contains("font-family: var(--material-typography-typeface-mono)")
			.xpect_contains("white-space: pre-wrap")
			.xpect_contains("word-break: break-word")
			.xpect_contains("max-height: 70vh")
			.xpect_contains("overflow-y: auto")
			.xpect_contains(".beet-console-error")
			.xpect_contains("color: var(--material-colors-error)")
			.xpect_contains(".beet-console-info")
			.xpect_contains("color: var(--material-colors-primary)");
	}
}
