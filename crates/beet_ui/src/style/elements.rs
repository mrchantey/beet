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
use crate::style::common_props::*;
use beet_core::prelude::*;

/// Faint grey used for de-emphasised elements (quotes, rules, h6).
fn faint() -> Color { Color::srgba(1., 1., 1., 0.4) }

/// All default prose element rules, in cascade order.
pub fn default_element_rules() -> Vec<Rule> {
	vec![
		heading("h1", Color::srgb(0.,    0.502, 0.   )),
		heading("h2", Color::srgb(0.,    0.502, 0.502)),
		heading("h3", Color::srgb(0.,    0.,    0.502)),
		heading("h4", Color::srgb(0.502, 0.,    0.502)),
		bold(&["h5"]),
		heading("h6", faint()),
		bold(&["strong", "b"]),
		italic(&["em", "i"]),
		strikethrough(&["del", "s"]),
		colored(&["code"], Color::srgb(0.502, 0.502, 0.)),
		colored(&["pre"],  Color::srgba(0.502, 0.502, 0., 0.4)),
		link(),
		blockquote(),
		colored(&["hr", "img"], faint()),
	]
}

fn tag_rule(tags: &[&str]) -> Rule {
	let selector = match tags {
		[tag] => Selector::tag(*tag),
		_ => Selector::AnyOf(tags.iter().map(|t| Selector::tag(*t)).collect()),
	};
	Rule::new().with_selector(selector)
}

fn heading(tag: &str, color: Color) -> Rule {
	tag_rule(&[tag])
		.with_value(ForegroundColor, color)
		.with_value(TextStyleProp, TextStyle::BOLD)
}

fn bold(tags: &[&str]) -> Rule {
	tag_rule(tags).with_value(TextStyleProp, TextStyle::BOLD)
}

fn italic(tags: &[&str]) -> Rule {
	tag_rule(tags).with_value(TextStyleProp, TextStyle::ITALIC)
}

fn strikethrough(tags: &[&str]) -> Rule {
	tag_rule(tags).with_value(DecorationLineProp, DecorationLine::line_through())
}

fn colored(tags: &[&str], color: Color) -> Rule {
	tag_rule(tags).with_value(ForegroundColor, color)
}

fn link() -> Rule {
	tag_rule(&["a"])
		.with_value(ForegroundColor, Color::srgb(0., 0., 0.502))
		.with_value(DecorationLineProp, DecorationLine::underline())
}

fn blockquote() -> Rule {
	tag_rule(&["blockquote"])
		.with_value(ForegroundColor, faint())
		.with_value(TextStyleProp, TextStyle::ITALIC)
}
