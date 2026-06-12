#![cfg_attr(rustfmt, rustfmt_skip)]
//! Default colour schemes for syntax highlighting tokens.
//!
//! [`default_scheme`] yields the token → colour declarations applied at the
//! [`Selector::Root`] level (the dark palette, the no-scheme fallback).
//! [`light_scheme`]/[`dark_scheme`] override those tokens under the
//! `.light-scheme`/`.dark-scheme` body class, so highlighting tracks the active
//! colour scheme like every other themed colour — without them the dark palette
//! (bright, near-white punctuation) is unreadable on a light background.
//!
//! [`class_rules`] binds each `.hl-<name>` class to its [`ForegroundColor`]
//! token, so an element matching `class="hl-<name>"` resolves a colour without
//! bespoke per-renderer wiring.

use crate::prelude::*;
use crate::style::common_props::ForegroundColor;
use crate::style::syntax::tokens as t;
use beet_core::prelude::*;

/// A syntax colour scheme grouped into semantic roles. Each highlight token maps
/// to one role (see [`declarations`]), so a scheme is a dozen colours rather than
/// forty, and the light/dark variants stay structurally identical.
struct SyntaxPalette {
	/// Control-flow keywords and macro-like special punctuation.
	keyword: Color,
	/// Functions, constructors, properties, tags, links, headings.
	function: Color,
	/// Types and built-in functions.
	r#type: Color,
	/// String literals, modules, raw markup.
	string: Color,
	/// Numbers, booleans, constants, escapes.
	number: Color,
	/// Comments and quotes (dimmed).
	comment: Color,
	/// Plain text, brackets, delimiters, variables, markup — the bulk of code.
	/// Must read clearly against the background (the light-mode bug was a
	/// near-white value here).
	punctuation: Color,
	/// Operators.
	operator: Color,
	/// Attributes and parameters.
	attribute: Color,
	/// Emphasis accents (bold, string escapes).
	accent: Color,
	/// Errors.
	error: Color,
	/// Embedded foreign syntax.
	embedded: Color,
}

/// The dark palette: bright tokens for a dark background. Also the
/// [`Selector::Root`] fallback used when no scheme class is present.
fn dark_palette() -> SyntaxPalette {
	SyntaxPalette {
		keyword:     Color::srgb(0.95, 0.45, 0.75),
		function:    Color::srgb(0.40, 0.75, 0.95),
		r#type:      Color::srgb(0.40, 0.85, 0.85),
		string:      Color::srgb(0.60, 0.85, 0.55),
		number:      Color::srgb(0.95, 0.55, 0.20),
		comment:     Color::srgba(0.55, 0.62, 0.55, 0.90),
		punctuation: Color::srgb(0.82, 0.83, 0.85),
		operator:    Color::srgb(0.90, 0.50, 0.70),
		attribute:   Color::srgb(0.85, 0.65, 0.30),
		accent:      Color::srgb(0.95, 0.85, 0.45),
		error:       Color::srgb(0.95, 0.30, 0.30),
		embedded:    Color::srgb(0.75, 0.55, 0.95),
	}
}

/// The light palette: dark, saturated tokens for a light background. Crucially
/// `punctuation` is near-black so brackets and plain code read on white.
fn light_palette() -> SyntaxPalette {
	SyntaxPalette {
		keyword:     Color::srgb(0.72, 0.11, 0.42),
		function:    Color::srgb(0.18, 0.33, 0.74),
		r#type:      Color::srgb(0.00, 0.42, 0.49),
		string:      Color::srgb(0.05, 0.46, 0.16),
		number:      Color::srgb(0.62, 0.30, 0.00),
		comment:     Color::srgba(0.40, 0.46, 0.40, 0.95),
		punctuation: Color::srgb(0.18, 0.20, 0.22),
		operator:    Color::srgb(0.63, 0.16, 0.44),
		attribute:   Color::srgb(0.46, 0.33, 0.00),
		accent:      Color::srgb(0.55, 0.40, 0.00),
		error:       Color::srgb(0.78, 0.00, 0.00),
		embedded:    Color::srgb(0.42, 0.18, 0.62),
	}
}

/// Map every highlight token to its palette role.
fn declarations(p: &SyntaxPalette) -> Vec<(TokenKey, TokenValue)> {
	Rule::new()
		.with_value(t::Attribute,            p.attribute)
		.with_value(t::Boolean,              p.number)
		.with_value(t::Comment,              p.comment)
		.with_value(t::CommentDocumentation, p.comment)
		.with_value(t::Constant,             p.number)
		.with_value(t::ConstantBuiltin,      p.number)
		.with_value(t::Constructor,          p.function)
		.with_value(t::Embedded,             p.embedded)
		.with_value(t::Error,                p.error)
		.with_value(t::Escape,               p.number)
		.with_value(t::Function,             p.function)
		.with_value(t::FunctionBuiltin,      p.r#type)
		.with_value(t::Keyword,              p.keyword)
		.with_value(t::Markup,               p.punctuation)
		.with_value(t::MarkupBold,           p.accent)
		.with_value(t::MarkupHeading,        p.function)
		.with_value(t::MarkupItalic,         p.punctuation)
		.with_value(t::MarkupLink,           p.function)
		.with_value(t::MarkupList,           p.punctuation)
		.with_value(t::MarkupQuote,          p.comment)
		.with_value(t::MarkupRaw,            p.string)
		.with_value(t::MarkupStrikethrough,  p.comment)
		.with_value(t::Module,               p.string)
		.with_value(t::Number,               p.number)
		.with_value(t::Operator,             p.operator)
		.with_value(t::Property,             p.function)
		.with_value(t::PropertyBuiltin,      p.function)
		.with_value(t::Punctuation,          p.punctuation)
		.with_value(t::PunctuationBracket,   p.punctuation)
		.with_value(t::PunctuationDelimiter, p.punctuation)
		.with_value(t::PunctuationSpecial,   p.keyword)
		.with_value(t::String,               p.string)
		.with_value(t::StringEscape,         p.accent)
		.with_value(t::StringRegexp,         p.number)
		.with_value(t::StringSpecial,        p.number)
		.with_value(t::StringSpecialSymbol,  p.number)
		.with_value(t::Tag,                  p.function)
		.with_value(t::Type,                 p.r#type)
		.with_value(t::TypeBuiltin,          p.r#type)
		.with_value(t::Variable,             p.punctuation)
		.with_value(t::VariableBuiltin,      p.number)
		.with_value(t::VariableMember,       p.function)
		.with_value(t::VariableParameter,    p.attribute)
		.into_iter()
		.collect()
}

/// The root-level fallback declarations (the dark palette), for contexts that
/// carry no `.light-scheme`/`.dark-scheme` class.
pub fn default_scheme() -> Vec<(TokenKey, TokenValue)> {
	declarations(&dark_palette())
}

/// Token values for a light background, gated on the `.light-scheme` body class.
pub fn light_scheme() -> Rule {
	Rule::new()
		.with_selector(Selector::class(classes::LIGHT_SCHEME))
		.with_extend(declarations(&light_palette()))
}

/// Token values for a dark background, gated on the `.dark-scheme` body class.
pub fn dark_scheme() -> Rule {
	Rule::new()
		.with_selector(Selector::class(classes::DARK_SCHEME))
		.with_extend(declarations(&dark_palette()))
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

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::style::StylePlugin;
	use crate::style::common_props::ForegroundColor;
	use crate::style::material::MaterialStylePlugin;
	use crate::style::material::classes;
	use beet_core::prelude::*;

	/// The resolved `.hl-punctuation` foreground under a given scheme class.
	fn punctuation_fg(scheme: ClassName) -> Color {
		let mut world =
			(MaterialStylePlugin::default(), StylePlugin).into_world();
		let parent = world
			.spawn((Element::new("div"), Classes::new([scheme])))
			.id();
		let span = world
			.spawn((
				Element::new("span"),
				Classes::new([ClassName::new_static("hl-punctuation")]),
				ChildOf(parent),
			))
			.id();
		world.with_state::<RuleSetQuery, _>(|query| {
			query
				.resolve(span, ForegroundColor, &mut default())
				.unwrap()
		})
	}

	fn luma(color: Color) -> f32 {
		let c = color.to_srgba();
		c.red + c.green + c.blue
	}

	/// Highlight tokens track the colour scheme: punctuation (the bulk of code,
	/// the light-mode bug's faint near-white) is dark on light and light on dark,
	/// so brackets read on either background.
	#[beet_core::test]
	fn punctuation_is_scheme_aware() {
		let light = punctuation_fg(classes::LIGHT_SCHEME);
		let dark = punctuation_fg(classes::DARK_SCHEME);
		// light scheme: near-black, readable on a light surface.
		(luma(light) < 1.0).xpect_true();
		// dark scheme: near-white, readable on a dark surface.
		(luma(dark) > 2.0).xpect_true();
		// and they genuinely differ (not the single fixed palette).
		(light != dark).xpect_true();
	}
}
