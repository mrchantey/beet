//! `RenderConsole` panel classes and their Material Design 3 rules.
//!
//! The panel and per-line level colors for the [`RenderConsole`] widget. The
//! line-level classes ([`CONSOLE_ERROR`]/[`CONSOLE_WARN`]/[`CONSOLE_INFO`]) are
//! set by `render_console.js` on each captured log line, so their selector
//! strings are the contract with that script and must not drift.
//!
//! [`RenderConsole`]: crate::prelude::RenderConsole
#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::*;
use crate::style::material::*;

// ── Class names ─────────────────────────────────────────────────────────────────

/// The scrollable monospace panel captured console output appends to.
pub const CONSOLE_PANEL: ClassName = ClassName::new_static("beet-console-panel");
/// A single captured log line; one per `console.*` call.
pub const CONSOLE_LINE: ClassName = ClassName::new_static("beet-console-line");
/// An `error`-level line, colored with the theme's error role.
pub const CONSOLE_ERROR: ClassName = ClassName::new_static("beet-console-error");
/// A `warn`-level line, colored with the theme's tertiary role.
pub const CONSOLE_WARN: ClassName = ClassName::new_static("beet-console-warn");
/// An `info`-level line, colored with the theme's primary role.
pub const CONSOLE_INFO: ClassName = ClassName::new_static("beet-console-info");

// ── Rules ─────────────────────────────────────────────────────────────────────

/// The console panel: a monospace, scrollable surface that wraps long lines yet
/// preserves their own breaks. Colors lean on the material surface roles, so it
/// recolors with the theme.
pub fn console_panel() -> Rule {
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
pub fn console_line() -> Rule {
	Rule::new()
		.with_selector(Selector::class(CONSOLE_LINE))
		.with_canonical(Display::Block)
}

/// `error`-level line color.
pub fn console_error() -> Rule {
	console_level(CONSOLE_ERROR, colors::Error)
}

/// `warn`-level line color.
pub fn console_warn() -> Rule {
	console_level(CONSOLE_WARN, colors::Tertiary)
}

/// `info`-level line color.
pub fn console_info() -> Rule {
	console_level(CONSOLE_INFO, colors::Primary)
}

/// A per-level line color rule, pointing `class`'s foreground at a color role.
fn console_level(class: ClassName, color: impl Into<Token>) -> Rule {
	Rule::new()
		.with_selector(Selector::class(class))
		.with_token(common_props::ForegroundColor, color).unwrap()
}

#[cfg(test)]
mod tests {
	use super::*;
	use beet_core::prelude::*;

	/// The console panel + level rules serialize to the expected CSS, exercising
	/// the `font-family`/`white-space: pre-wrap`/`word-break` props added for it.
	#[beet_core::test]
	fn console_rules_css() {
		let css = CssBuilder::default()
			.with_format_variables(FormatVariables::short())
			.build(
				&default_token_map().with_extend(common_props::token_map()),
				&RuleSet::new(Rule::new()).with_rules(vec![
					console_panel(),
					console_line(),
					console_error(),
					console_warn(),
					console_info(),
				]),
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
