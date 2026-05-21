#![cfg_attr(rustfmt, rustfmt_skip)]
//! Design tokens (one per tree-sitter capture name) for syntax highlighting.
//!
//! Capture names like `string.escape` are mapped to PascalCase variants by
//! removing dots, eg `StringEscape`. The `capture_name` const on each token
//! provides the original dotted form for tree-sitter dispatch.

use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;

/// Registers every syntax highlight token with the [`CssTokenMap`].
pub fn token_map() -> CssTokenMap {
	CssTokenMap::default()
		.insert(Attribute)
		.insert(Boolean)
		.insert(Comment)
		.insert(CommentDocumentation)
		.insert(Constant)
		.insert(ConstantBuiltin)
		.insert(Constructor)
		.insert(Embedded)
		.insert(Error)
		.insert(Escape)
		.insert(Function)
		.insert(FunctionBuiltin)
		.insert(Keyword)
		.insert(Markup)
		.insert(MarkupBold)
		.insert(MarkupHeading)
		.insert(MarkupItalic)
		.insert(MarkupLink)
		.insert(MarkupList)
		.insert(MarkupQuote)
		.insert(MarkupRaw)
		.insert(MarkupStrikethrough)
		.insert(Module)
		.insert(Number)
		.insert(Operator)
		.insert(Property)
		.insert(PropertyBuiltin)
		.insert(Punctuation)
		.insert(PunctuationBracket)
		.insert(PunctuationDelimiter)
		.insert(PunctuationSpecial)
		.insert(String)
		.insert(StringEscape)
		.insert(StringRegexp)
		.insert(StringSpecial)
		.insert(StringSpecialSymbol)
		.insert(Tag)
		.insert(Type)
		.insert(TypeBuiltin)
		.insert(Variable)
		.insert(VariableBuiltin)
		.insert(VariableMember)
		.insert(VariableParameter)
}

/// Ordered list of recognised tree-sitter capture names. Order is important
/// for the longest-match rule used by tree-sitter's highlight resolution.
pub fn recognised_names() -> &'static [&'static str] {
	&[
		"attribute",
		"boolean",
		"comment",
		"comment.documentation",
		"constant",
		"constant.builtin",
		"constructor",
		"embedded",
		"error",
		"escape",
		"function",
		"function.builtin",
		"keyword",
		"markup",
		"markup.bold",
		"markup.heading",
		"markup.italic",
		"markup.link",
		"markup.list",
		"markup.quote",
		"markup.raw",
		"markup.strikethrough",
		"module",
		"number",
		"operator",
		"property",
		"property.builtin",
		"punctuation",
		"punctuation.bracket",
		"punctuation.delimiter",
		"punctuation.special",
		"string",
		"string.escape",
		"string.regexp",
		"string.special",
		"string.special.symbol",
		"tag",
		"type",
		"type.builtin",
		"variable",
		"variable.builtin",
		"variable.member",
		"variable.parameter",
	]
}

/// Maps a recognised capture name to its [`CssVariable`] for foreground colour.
pub fn css_variable_for(name: &str) -> Option<CssVariable> {
	let key = match name {
		"attribute" => Attribute::token_key(),
		"boolean" => Boolean::token_key(),
		"comment" => Comment::token_key(),
		"comment.documentation" => CommentDocumentation::token_key(),
		"constant" => Constant::token_key(),
		"constant.builtin" => ConstantBuiltin::token_key(),
		"constructor" => Constructor::token_key(),
		"embedded" => Embedded::token_key(),
		"error" => Error::token_key(),
		"escape" => Escape::token_key(),
		"function" => Function::token_key(),
		"function.builtin" => FunctionBuiltin::token_key(),
		"keyword" => Keyword::token_key(),
		"markup" => Markup::token_key(),
		"markup.bold" => MarkupBold::token_key(),
		"markup.heading" => MarkupHeading::token_key(),
		"markup.italic" => MarkupItalic::token_key(),
		"markup.link" => MarkupLink::token_key(),
		"markup.list" => MarkupList::token_key(),
		"markup.quote" => MarkupQuote::token_key(),
		"markup.raw" => MarkupRaw::token_key(),
		"markup.strikethrough" => MarkupStrikethrough::token_key(),
		"module" => Module::token_key(),
		"number" => Number::token_key(),
		"operator" => Operator::token_key(),
		"property" => Property::token_key(),
		"property.builtin" => PropertyBuiltin::token_key(),
		"punctuation" => Punctuation::token_key(),
		"punctuation.bracket" => PunctuationBracket::token_key(),
		"punctuation.delimiter" => PunctuationDelimiter::token_key(),
		"punctuation.special" => PunctuationSpecial::token_key(),
		"string" => String::token_key(),
		"string.escape" => StringEscape::token_key(),
		"string.regexp" => StringRegexp::token_key(),
		"string.special" => StringSpecial::token_key(),
		"string.special.symbol" => StringSpecialSymbol::token_key(),
		"tag" => Tag::token_key(),
		"type" => Type::token_key(),
		"type.builtin" => TypeBuiltin::token_key(),
		"variable" => Variable::token_key(),
		"variable.builtin" => VariableBuiltin::token_key(),
		"variable.member" => VariableMember::token_key(),
		"variable.parameter" => VariableParameter::token_key(),
		_ => return None,
	};
	Some(CssVariable::from_token_key(&key))
}

// ── Attributes & metadata ─────────────────────────────────────────────────
css_variable!(Attribute, Color);

// ── Literals ──────────────────────────────────────────────────────────────
css_variable!(Boolean, Color);
css_variable!(Number, Color);

// ── Comments ──────────────────────────────────────────────────────────────
css_variable!(Comment, Color);
css_variable!(CommentDocumentation, Color);

// ── Constants & constructors ──────────────────────────────────────────────
css_variable!(Constant, Color);
css_variable!(ConstantBuiltin, Color);
css_variable!(Constructor, Color);

// ── Special ──────────────────────────────────────────────────────────────
css_variable!(Embedded, Color);
css_variable!(Error, Color);
css_variable!(Escape, Color);

// ── Functions ─────────────────────────────────────────────────────────────
css_variable!(Function, Color);
css_variable!(FunctionBuiltin, Color);

// ── Keywords ──────────────────────────────────────────────────────────────
css_variable!(Keyword, Color);

// ── Markup (for markdown / html highlight) ────────────────────────────────
css_variable!(Markup, Color);
css_variable!(MarkupBold, Color);
css_variable!(MarkupHeading, Color);
css_variable!(MarkupItalic, Color);
css_variable!(MarkupLink, Color);
css_variable!(MarkupList, Color);
css_variable!(MarkupQuote, Color);
css_variable!(MarkupRaw, Color);
css_variable!(MarkupStrikethrough, Color);

// ── Modules ───────────────────────────────────────────────────────────────
css_variable!(Module, Color);

// ── Operators ─────────────────────────────────────────────────────────────
css_variable!(Operator, Color);

// ── Properties ────────────────────────────────────────────────────────────
css_variable!(Property, Color);
css_variable!(PropertyBuiltin, Color);

// ── Punctuation ───────────────────────────────────────────────────────────
css_variable!(Punctuation, Color);
css_variable!(PunctuationBracket, Color);
css_variable!(PunctuationDelimiter, Color);
css_variable!(PunctuationSpecial, Color);

// ── Strings ───────────────────────────────────────────────────────────────
css_variable!(String, Color);
css_variable!(StringEscape, Color);
css_variable!(StringRegexp, Color);
css_variable!(StringSpecial, Color);
css_variable!(StringSpecialSymbol, Color);

// ── Tags ──────────────────────────────────────────────────────────────────
css_variable!(Tag, Color);

// ── Types ─────────────────────────────────────────────────────────────────
css_variable!(Type, Color);
css_variable!(TypeBuiltin, Color);

// ── Variables ─────────────────────────────────────────────────────────────
css_variable!(Variable, Color);
css_variable!(VariableBuiltin, Color);
css_variable!(VariableMember, Color);
css_variable!(VariableParameter, Color);
