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
use crate::style::Display;
use crate::style::FontStyle;
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
		// ── Block structure ──
		block_spaced(&["p", "ul", "ol"]),
		block(&["li", "div"]),
		heading("h1", Color::srgb(0.,    0.502, 0.   )),
		heading("h2", Color::srgb(0.,    0.502, 0.502)),
		heading("h3", Color::srgb(0.,    0.,    0.502)),
		heading("h4", Color::srgb(0.502, 0.,    0.502)),
		block_spaced(&["h5"]).with_value(FontWeightProp, FontWeight::Bold),
		heading("h6", faint()),
		blockquote(),
		block_spaced(&["pre"])
			.with_value(WhiteSpaceProp, WhiteSpace::Pre)
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

/// A rule forcing `display: block` on the given tags.
fn block(tags: &[&str]) -> Rule {
	Rule::tags(tags).with_value(DisplayProp, Display::Block)
}

/// A `display: block` rule that also reserves a [`block_gap`] below, separating
/// the element from the following block.
fn block_spaced(tags: &[&str]) -> Rule {
	block(tags).with_value(MarginProp, block_gap())
}

/// A rule forcing `display: inline` on the given tags.
fn inline(tags: &[&str]) -> Rule {
	Rule::tags(tags).with_value(DisplayProp, Display::Inline)
}

fn heading(tag: &str, color: Color) -> Rule {
	block_spaced(&[tag])
		.with_value(ForegroundColor, color)
		.with_value(FontWeightProp, FontWeight::Bold)
}

fn bold(tags: &[&str]) -> Rule {
	inline(tags).with_value(FontWeightProp, FontWeight::Bold)
}

fn italic(tags: &[&str]) -> Rule {
	inline(tags).with_value(FontStyleProp, FontStyle::Italic)
}

fn strikethrough(tags: &[&str]) -> Rule {
	inline(tags).with_value(DecorationLineProp, DecorationLine::line_through())
}

fn link() -> Rule {
	inline(&["a"])
		.with_value(ForegroundColor, Color::srgb(0., 0., 0.502))
		.with_value(DecorationLineProp, DecorationLine::underline())
}

fn blockquote() -> Rule {
	block_spaced(&["blockquote"])
		.with_value(ForegroundColor, faint())
		.with_value(FontStyleProp, FontStyle::Italic)
}
