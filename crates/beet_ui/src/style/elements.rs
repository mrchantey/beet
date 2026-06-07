#![cfg_attr(rustfmt, rustfmt_skip)]
//! Default presentation rules for prose HTML elements.
//!
//! These are the terminal/char-cell "user agent stylesheet": a heading is
//! bold and coloured, emphasis is italic, links are underlined, and so on.
//! They are plain [`Rule`]s keyed on tag selectors so all styling flows
//! through the single [`RuleSet`] cascade rather than any renderer-local
//! lookup. Registered by [`StylePlugin`] so they are present when
//! [`resolve_styles`] runs in the [`PostParseTree`] schedule.

use crate::prelude::*;
use crate::style::Cursor;
use crate::style::Display;
use crate::style::FontStyle;
use crate::style::ListStyle;
use crate::style::FontWeight;
use crate::style::Length;
use crate::style::Spacing;
use crate::style::WhiteSpace;
use crate::style::common_props::*;
use beet_core::prelude::*;

/// Faint grey used for de-emphasised elements (quotes, rules, h6).
fn faint() -> Color { Color::srgba(1., 1., 1., 0.4) }

/// Neutral soft white for code text with no syntax highlight, matching the
/// default syntax theme's plain-token colour. Uncaptured runs inside a code
/// block (eg JSON keys, HTML text) inherit this rather than a tinted hue.
fn code_fg() -> Color { Color::srgb(0.85, 0.85, 0.85) }

/// One blank row below a block, separating it from the next (the `\n\n` between
/// markdown blocks). `li` items stay tight, so they opt out.
fn block_gap() -> Spacing {
	Spacing {
		bottom: Length::Rem(1.),
		..Spacing::DEFAULT
	}
}

/// All default prose element rules, in cascade order.
///
/// Each rule also declares its `display` so the charcell layout treats prose
/// structure correctly: headings/paragraphs/lists/quotes/code blocks flow as
/// blocks, while emphasis/links/inline code flow inline.
pub fn default_element_rules() -> Vec<Rule> {
	vec![
		non_visual_rule(),
		// ── Block structure ──
		block_spaced(&["p", "ul", "ol"]),
		block(&["li", "div"]),
		// disclosure: a `<details>` is a block, its `<summary>` a clickable block
		// header (pointer cursor on the header only). Generic (not sidebar-
		// specific) so any disclosure lays out the same on both targets; the
		// sidebar layers classes on top.
		block(&["details"]),
		block(&["summary"]).with_value(CursorProp, Cursor::Pointer),
		// navigation lists carry no markers (the sidebar is nav, not prose). CSS
		// `list-style-type` is inherited, so `none` on the `<nav>` strips bullets
		// from every list item inside it on both targets — the charcell decorator
		// reads the resolved value, the web honors it directly.
		nav_list_style(),
		// headings are plain bold on every target; the terminal-only accent
		// hue comes from `classes::terminal_headings`, keeping these user-agent
		// defaults free of any hardcoded palette.
		heading("h1"),
		heading("h2"),
		heading("h3"),
		heading("h4"),
		heading("h5"),
		heading("h6"),
		blockquote(),
		block_spaced(&["pre"])
			.with_canonical(WhiteSpace::Pre)
			.with_value(ForegroundColor, code_fg()),
		block_spaced(&["hr"]).with_value(ForegroundColor, faint()),
		// ── Inline formatting ──
		bold(&["strong", "b"]),
		italic(&["em", "i"]),
		strikethrough(&["del", "s"]),
		inline(&["code"]).with_value(ForegroundColor, code_fg()),
		inline(&["span"]),
		link(),
		inline(&["img"]).with_value(ForegroundColor, faint()),
	]
}

/// User-agent rule removing metadata and scripting tags from layout via
/// `display: none`, so visual renderers omit them.
///
/// Note this is a strict subset of [`NON_VISUAL_TAGS`]: the embedded-media tags
/// (`iframe`/`object`/`embed`) are visual on the web, so they are *not* hidden
/// here. The charcell walker still skips them via [`is_non_visual`], so they
/// render on the web but not the terminal.
///
/// Kept separate from the prose [`default_element_rules`] so theme rule sets (eg
/// Material) can include it without pulling in prose props that need their own
/// CSS resolvers.
pub fn non_visual_rule() -> Rule {
	Rule::tags(&[
		"head", "script", "style", "template", "noscript", "meta", "link",
		"title", "base",
	])
	.with_canonical(Display::None)
}

/// A rule forcing `display: block` on the given tags.
fn block(tags: &[&str]) -> Rule {
	Rule::tags(tags).with_canonical(Display::Block)
}

/// `<nav>` strips list markers from its descendant lists (inherited
/// `list-style-type: none`), so navigation reads as links, not a bulleted list.
fn nav_list_style() -> Rule {
	Rule::tags(&["nav"]).with_canonical(ListStyle::None)
}

/// A `display: block` rule that also reserves a [`block_gap`] below, separating
/// the element from the following block.
fn block_spaced(tags: &[&str]) -> Rule {
	block(tags).with_value(MarginProp, block_gap())
}

/// A rule forcing `display: inline` on the given tags.
fn inline(tags: &[&str]) -> Rule {
	Rule::tags(tags).with_canonical(Display::Inline)
}

fn heading(tag: &str) -> Rule {
	block_spaced(&[tag]).with_canonical(FontWeight::Bold)
}

fn bold(tags: &[&str]) -> Rule {
	inline(tags).with_canonical(FontWeight::Bold)
}

fn italic(tags: &[&str]) -> Rule {
	inline(tags).with_canonical(FontStyle::Italic)
}

fn strikethrough(tags: &[&str]) -> Rule {
	inline(tags).with_canonical(DecorationLine::line_through())
}

fn link() -> Rule {
	inline(&["a"])
		.with_value(ForegroundColor, Color::srgb(0., 0., 0.502))
		.with_canonical(DecorationLine::underline())
}

// A `blockquote` is a block with the usual gap below; its callout look (lifted
// surface, primary left rule) comes from the theme's `blockquote_prose` rule.
fn blockquote() -> Rule {
	block_spaced(&["blockquote"])
}
