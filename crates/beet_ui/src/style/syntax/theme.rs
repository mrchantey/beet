#![cfg_attr(rustfmt, rustfmt_skip)]
//! Default colour schemes for syntax highlighting tokens.
//!
//! Provides one warm light scheme and one cool dark scheme. Use
//! [`default_scheme`] for a colour set that adapts to both.

use crate::prelude::*;
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
