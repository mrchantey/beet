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
pub(crate) fn token_map() -> CssTokenMap {
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
