#![cfg_attr(rustfmt, rustfmt_skip)]
//! Default colour schemes for syntax highlighting tokens.
//!
//! [`default_scheme`] yields the token → colour declarations applied at
//! the [`Selector::Root`] level. [`class_rules`] yields one rule per
//! recognised capture name, binding `.hl-<name>` to the corresponding
//! foreground colour token. Together they let any element matching
//! `class="hl-<name>"` resolve [`ForegroundColor`] without bespoke
//! per-renderer wiring.

use crate::prelude::*;
use crate::style::common_props::ForegroundColor;
use crate::style::syntax::tokens as t;
use beet_core::prelude::*;

/// Default colour scheme suitable for terminals with either background.
///
/// Picked to be readable on most terminal palettes without depending on
/// the material seed colour. Override individual values by inserting
/// later rules into the [`RuleSet`].
pub fn default_scheme() -> Vec<(TokenKey, TokenValue)> {
	Rule::new()
		.with_value(t::Attribute,            Color::srgb(0.85, 0.65, 0.30))
		.with_value(t::Boolean,              Color::srgb(0.95, 0.55, 0.20))
		.with_value(t::Comment,              Color::srgba(0.55, 0.60, 0.55, 0.85))
		.with_value(t::CommentDocumentation, Color::srgba(0.55, 0.70, 0.55, 0.95))
		.with_value(t::Constant,             Color::srgb(0.95, 0.55, 0.20))
		.with_value(t::ConstantBuiltin,      Color::srgb(0.95, 0.55, 0.20))
		.with_value(t::Constructor,          Color::srgb(0.40, 0.75, 0.95))
		.with_value(t::Embedded,             Color::srgb(0.75, 0.55, 0.95))
		.with_value(t::Error,                Color::srgb(0.95, 0.30, 0.30))
		.with_value(t::Escape,               Color::srgb(0.95, 0.55, 0.20))
		.with_value(t::Function,             Color::srgb(0.40, 0.75, 0.95))
		.with_value(t::FunctionBuiltin,      Color::srgb(0.40, 0.85, 0.85))
		.with_value(t::Keyword,              Color::srgb(0.95, 0.45, 0.75))
		.with_value(t::Markup,               Color::srgb(0.85, 0.85, 0.85))
		.with_value(t::MarkupBold,           Color::srgb(0.95, 0.85, 0.45))
		.with_value(t::MarkupHeading,        Color::srgb(0.40, 0.75, 0.95))
		.with_value(t::MarkupItalic,         Color::srgb(0.85, 0.85, 0.85))
		.with_value(t::MarkupLink,           Color::srgb(0.40, 0.75, 0.95))
		.with_value(t::MarkupList,           Color::srgb(0.85, 0.85, 0.85))
		.with_value(t::MarkupQuote,          Color::srgba(0.80, 0.80, 0.80, 0.85))
		.with_value(t::MarkupRaw,            Color::srgb(0.60, 0.85, 0.55))
		.with_value(t::MarkupStrikethrough,  Color::srgba(0.70, 0.70, 0.70, 0.85))
		.with_value(t::Module,               Color::srgb(0.60, 0.85, 0.55))
		.with_value(t::Number,               Color::srgb(0.95, 0.55, 0.20))
		.with_value(t::Operator,             Color::srgb(0.90, 0.50, 0.70))
		.with_value(t::Property,             Color::srgb(0.40, 0.75, 0.95))
		.with_value(t::PropertyBuiltin,      Color::srgb(0.40, 0.75, 0.95))
		.with_value(t::Punctuation,          Color::srgba(0.85, 0.85, 0.85, 0.85))
		.with_value(t::PunctuationBracket,   Color::srgba(0.85, 0.85, 0.85, 0.85))
		.with_value(t::PunctuationDelimiter, Color::srgba(0.85, 0.85, 0.85, 0.85))
		.with_value(t::PunctuationSpecial,   Color::srgb(0.95, 0.45, 0.75))
		.with_value(t::String,               Color::srgb(0.60, 0.85, 0.55))
		.with_value(t::StringEscape,         Color::srgb(0.95, 0.85, 0.45))
		.with_value(t::StringRegexp,         Color::srgb(0.95, 0.55, 0.20))
		.with_value(t::StringSpecial,        Color::srgb(0.95, 0.55, 0.20))
		.with_value(t::StringSpecialSymbol,  Color::srgb(0.95, 0.55, 0.20))
		.with_value(t::Tag,                  Color::srgb(0.40, 0.75, 0.95))
		.with_value(t::Type,                 Color::srgb(0.40, 0.85, 0.85))
		.with_value(t::TypeBuiltin,          Color::srgb(0.40, 0.85, 0.85))
		.with_value(t::Variable,             Color::srgb(0.85, 0.85, 0.85))
		.with_value(t::VariableBuiltin,      Color::srgb(0.95, 0.55, 0.20))
		.with_value(t::VariableMember,       Color::srgb(0.40, 0.75, 0.95))
		.with_value(t::VariableParameter,    Color::srgb(0.85, 0.65, 0.30))
		.into_iter()
		.collect()
}

/// One [`Rule`] per syntax token, binding the `.hl-<name>` class to a
/// [`ForegroundColor`] declaration that redirects to the matching token.
///
/// Capture names that contain a dot (eg `string.escape`) are written
/// verbatim into the class name, matching the output of
/// [`apply_syntax_highlighting`](crate::parse::apply_syntax_highlighting).
pub fn class_rules() -> Vec<Rule> {
	vec![
		hl_rule("attribute",             t::Attribute),
		hl_rule("boolean",               t::Boolean),
		hl_rule("comment",               t::Comment),
		hl_rule("comment.documentation", t::CommentDocumentation),
		hl_rule("constant",              t::Constant),
		hl_rule("constant.builtin",      t::ConstantBuiltin),
		hl_rule("constructor",           t::Constructor),
		hl_rule("embedded",              t::Embedded),
		hl_rule("error",                 t::Error),
		hl_rule("escape",                t::Escape),
		hl_rule("function",              t::Function),
		hl_rule("function.builtin",      t::FunctionBuiltin),
		hl_rule("keyword",               t::Keyword),
		hl_rule("markup",                t::Markup),
		hl_rule("markup.bold",           t::MarkupBold),
		hl_rule("markup.heading",        t::MarkupHeading),
		hl_rule("markup.italic",         t::MarkupItalic),
		hl_rule("markup.link",           t::MarkupLink),
		hl_rule("markup.list",           t::MarkupList),
		hl_rule("markup.quote",          t::MarkupQuote),
		hl_rule("markup.raw",            t::MarkupRaw),
		hl_rule("markup.strikethrough",  t::MarkupStrikethrough),
		hl_rule("module",                t::Module),
		hl_rule("number",                t::Number),
		hl_rule("operator",              t::Operator),
		hl_rule("property",              t::Property),
		hl_rule("property.builtin",      t::PropertyBuiltin),
		hl_rule("punctuation",           t::Punctuation),
		hl_rule("punctuation.bracket",   t::PunctuationBracket),
		hl_rule("punctuation.delimiter", t::PunctuationDelimiter),
		hl_rule("punctuation.special",   t::PunctuationSpecial),
		hl_rule("string",                t::String),
		hl_rule("string.escape",         t::StringEscape),
		hl_rule("string.regexp",         t::StringRegexp),
		hl_rule("string.special",        t::StringSpecial),
		hl_rule("string.special.symbol", t::StringSpecialSymbol),
		hl_rule("tag",                   t::Tag),
		hl_rule("type",                  t::Type),
		hl_rule("type.builtin",          t::TypeBuiltin),
		hl_rule("variable",              t::Variable),
		hl_rule("variable.builtin",      t::VariableBuiltin),
		hl_rule("variable.member",       t::VariableMember),
		hl_rule("variable.parameter",    t::VariableParameter),
	]
}

fn hl_rule(name: &str, token: impl Into<Token>) -> Rule {
	Rule::new()
		.with_selector(Selector::class(format!("hl-{name}")))
		.with_token(ForegroundColor, token)
		.expect("ForegroundColor schema matches syntax token schema")
}
