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
use crate::style::WhiteSpace;
use crate::style::common_props::*;
use beet_core::prelude::*;

/// Faint grey used for de-emphasised elements (quotes, rules, h6).
fn faint() -> Color { Color::srgba(1., 1., 1., 0.4) }

/// All default prose element rules, in cascade order.
///
/// Each rule also declares its `display` so the charcell layout treats prose
/// structure correctly: headings/paragraphs/lists/quotes/code blocks flow as
/// blocks, while emphasis/links/inline code flow inline.
pub fn default_element_rules() -> Vec<Rule> {
	vec![
		// ── Block structure ──
		block(&["p", "ul", "ol", "li", "div"]),
		heading("h1", Color::srgb(0.,    0.502, 0.   )),
		heading("h2", Color::srgb(0.,    0.502, 0.502)),
		heading("h3", Color::srgb(0.,    0.,    0.502)),
		heading("h4", Color::srgb(0.502, 0.,    0.502)),
		block(&["h5"]).with_value(FontWeightProp, FontWeight::Bold),
		heading("h6", faint()),
		blockquote(),
		block(&["pre"])
			.with_value(WhiteSpaceProp, WhiteSpace::Pre)
			.with_value(ForegroundColor, Color::srgba(0.502, 0.502, 0., 0.4)),
		block(&["hr"]).with_value(ForegroundColor, faint()),
		// ── Inline formatting ──
		bold(&["strong", "b"]),
		italic(&["em", "i"]),
		strikethrough(&["del", "s"]),
		inline(&["code"]).with_value(ForegroundColor, Color::srgb(0.502, 0.502, 0.)),
		inline(&["span"]),
		link(),
		inline(&["img"]).with_value(ForegroundColor, faint()),
	]
}

fn tag_rule(tags: &[&str]) -> Rule {
	let selector = match tags {
		[tag] => Selector::tag(*tag),
		_ => Selector::AnyOf(tags.iter().map(|t| Selector::tag(*t)).collect()),
	};
	Rule::new().with_selector(selector)
}

/// A rule forcing `display: block` on the given tags.
fn block(tags: &[&str]) -> Rule {
	tag_rule(tags).with_value(DisplayProp, Display::Block)
}

/// A rule forcing `display: inline` on the given tags.
fn inline(tags: &[&str]) -> Rule {
	tag_rule(tags).with_value(DisplayProp, Display::Inline)
}

fn heading(tag: &str, color: Color) -> Rule {
	block(&[tag])
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
	block(&["blockquote"])
		.with_value(ForegroundColor, faint())
		.with_value(FontStyleProp, FontStyle::Italic)
}
