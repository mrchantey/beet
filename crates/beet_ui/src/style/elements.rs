#![cfg_attr(rustfmt, rustfmt_skip)]
//! Default presentation rules for prose HTML elements.
//!
//! These are the terminal/char-cell "user agent stylesheet": emphasis is
//! italic, links are underlined, headings are spaced blocks, and so on.
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
		block(&["div"]),
		// a list item is block-level on both targets but keeps its web marker via
		// `display: list-item` (the charcell decorator draws the marker itself).
		list_item(&["li"]),
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
		// headings are plain spaced blocks: the web keeps its native bold, the
		// terminal-only accent hue comes from `classes::terminal_headings`, and
		// the charcell wide-glyph render (fullwidth/block, `FontScale`) hardcodes
		// bold itself, so no user-agent weight is set here.
		block_spaced(&["h1", "h2", "h3", "h4", "h5", "h6"]),
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
		// the remaining inline text-level tags carry no special default look
		// beyond flowing inline; without this they default to `display: block`
		// and break onto their own full-width line mid-sentence.
		inline(&[
			"mark", "small", "sub", "sup", "q", "cite", "abbr", "time",
			"samp", "kbd", "var", "dfn", "data", "bdi", "bdo", "wbr",
		]),
		// inserted text mirrors deleted text, underlined rather than struck
		inline(&["ins", "u"]).with_canonical(DecorationLine::underline()),
		link(),
		// an `<img>` that can't render in the terminal falls back to its alt
		// placeholder as a link (the charcell decorator gives it a `Marker` +
		// `Hyperlink`), so it styles, hovers, and clicks like an `<a>`. The link
		// colour comes from the theme token (`link_fallback_prose`,
		// `colors::Primary`); only the underline is a user-agent default here.
		// Terminal-gated: on the web an `<img>` is its picture, never a link. A
		// raster-backed `<img>` overlays this with its block box (the `graphics`
		// rule below) and is likewise its picture, not a link.
		inline(&["img"])
			.with_media(MediaQuery::Terminal)
			.with_canonical(DecorationLine::underline()),
		// an embedded `<iframe>` can't render in the terminal; the charcell
		// decorator collapses it to a titled OSC-8 link (its `Marker` text +
		// `Hyperlink`, painted as a link by `paint_text`), underlined as a block
		// on its own line. Web serializes the real iframe and ignores this.
		block_spaced(&["iframe"]).with_canonical(DecorationLine::underline()),
		// a closed `<select>` shows only its value label (a charcell `Marker`);
		// its options never lay out — the select dropdown spawns its own rows.
		// Terminal-gated: the web's native select needs its options visible.
		Rule::tags(&["option", "optgroup"])
			.with_media(MediaQuery::Terminal)
			.with_canonical(Display::None),
		// an `<img>` backed by a kitty-graphics raster (the attach system sets
		// the `graphics` state) gets its own block box sized by the raster;
		// the inline alt-text presentation applies everywhere else.
		Rule::new()
			.with_selector(Selector::AllOf(vec![
				Selector::tag("img"),
				Selector::state(graphics_state()),
			]))
			.with_media(MediaQuery::Terminal)
			.with_canonical(Display::Block),
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

/// A rule forcing `display: list-item` on the given tags, so the web keeps the
/// element's list marker. Charcell lays it out as a block.
fn list_item(tags: &[&str]) -> Rule {
	Rule::tags(tags).with_canonical(Display::ListItem)
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

fn bold(tags: &[&str]) -> Rule {
	inline(tags).with_canonical(FontWeight::Bold)
}

fn italic(tags: &[&str]) -> Rule {
	inline(tags).with_canonical(FontStyle::Italic)
}

fn strikethrough(tags: &[&str]) -> Rule {
	inline(tags).with_canonical(DecorationLine::line_through())
}

// the user-agent hyperlink colour is a plain fallback for token-less renders; the
// Material theme overrides it with its `Primary` accent token, see `link_prose`
// (and `link_fallback_prose` for the terminal's `<img>`/`<iframe>` link fallbacks).
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
