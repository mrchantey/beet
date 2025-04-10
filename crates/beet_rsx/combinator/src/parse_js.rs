use crate::prelude::*;
use combine::ParseResult;
use combine::Parser;
use combine::Stream;
use combine::char::alpha_num;
use combine::char::letter;
use combine::char::space;
use combine::char::string;
use combine::choice;
use combine::combinator::between;
use combine::combinator::many;
use combine::combinator::many1;
use combine::combinator::none_of;
use combine::combinator::parser;
use combine::combinator::skip_many;
use combine::combinator::token;
use combine::combinator::r#try;
use combine::combinator::value;

pub fn js_identifier_start<I>(input: I) -> ParseResult<JSIdentifierStart, I>
where
	I: Stream<Item = char>,
{
	choice!(r#try(letter()), parser(identifier_non_alpha_numeric))
		.map(JSIdentifierStart)
		.parse_stream(input)
}

pub fn js_identifier_part<I>(input: I) -> ParseResult<JSIdentifierPart, I>
where
	I: Stream<Item = char>,
{
	many1(choice!(
		r#try(alpha_num()),
		parser(identifier_non_alpha_numeric)
	))
	.map(JSIdentifierPart)
	.parse_stream(input)
}

pub fn js_boolean<I>(input: I) -> ParseResult<JSBool, I>
where
	I: Stream<Item = char>,
{
	choice!(r#try(string("true")), string("false"))
		.map(|s| match s {
			"true" => JSBool(true),
			"false" => JSBool(false),
			_ => panic!("Unsupported boolean string representation"),
		})
		.parse_stream(input)
}

pub fn js_number<I>(input: I) -> ParseResult<JSNumber, I>
where
	I: Stream<Item = char>,
{
	choice!(
		r#try(parser(float_exp).map(JSNumber)),
		r#try(parser(float_simple).map(JSNumber)),
		parser(integer).map(|v| v as f64).map(JSNumber)
	)
	.parse_stream(input)
}

pub fn js_double_string_character<I>(
	input: I,
) -> ParseResult<JSDoubleStringCharacter, I>
where
	I: Stream<Item = char>,
{
	choice!(r#try(parser(escaped_character)), none_of("\"".chars()))
		.map(JSDoubleStringCharacter)
		.parse_stream(input)
}

pub fn js_single_string_character<I>(
	input: I,
) -> ParseResult<JSSingleStringCharacter, I>
where
	I: Stream<Item = char>,
{
	choice!(r#try(parser(escaped_character)), none_of("'".chars()))
		.map(JSSingleStringCharacter)
		.parse_stream(input)
}

pub fn js_double_string_characters<I>(
	input: I,
) -> ParseResult<JSDoubleStringCharacters, I>
where
	I: Stream<Item = char>,
{
	between(
		token('"'),
		token('"'),
		many(parser(js_double_string_character)),
	)
	.parse_stream(input)
}

pub fn js_single_string_characters<I>(
	input: I,
) -> ParseResult<JSSingleStringCharacters, I>
where
	I: Stream<Item = char>,
{
	between(
		token('\''),
		token('\''),
		many(parser(js_single_string_character)),
	)
	.parse_stream(input)
}

pub fn js_single_line_comment<I>(input: I) -> ParseResult<(), I>
where
	I: Stream<Item = char>,
{
	between(string("//"), token('\n'), skip_many(none_of("\n".chars())))
		.parse_stream(input)
}

pub fn js_multi_line_comment<I>(input: I) -> ParseResult<(), I>
where
	I: Stream<Item = char>,
{
	between(
		string("/*"),
		string("*/"),
		skip_many(choice!(
			r#try((token('*'), none_of("/".chars())).with(value(()))),
			none_of("*".chars()).with(value(()))
		)),
	)
	.parse_stream(input)
}

pub fn js_comment<I>(input: I) -> ParseResult<(), I>
where
	I: Stream<Item = char>,
{
	choice!(
		r#try(parser(js_single_line_comment)),
		parser(js_multi_line_comment)
	)
	.with(value(()))
	.parse_stream(input)
}

pub fn js_whitespace<I>(input: I) -> ParseResult<(), I>
where
	I: Stream<Item = char>,
{
	skip_many(choice!(r#try(parser(js_comment)), space().with(value(()))))
		.with(value(()))
		.parse_stream(input)
}

#[cfg(test)]
mod tests {
	use super::parser as p;
	use super::*;

	#[test]
	pub fn test_js_identifier_start() {
		assert_eq!(parser(js_identifier_start).parse("").is_err(), true);
		assert_eq!(parser(js_identifier_start).parse(" ").is_err(), true);
		assert_eq!(
			parser(js_identifier_start).parse("_foobar").unwrap(),
			('_'.into(), "foobar")
		);
		assert_eq!(
			parser(js_identifier_start).parse("$foobar").unwrap(),
			('$'.into(), "foobar")
		);
		assert_eq!(
			parser(js_identifier_start).parse("afoobar").unwrap(),
			('a'.into(), "foobar")
		);
		assert_eq!(parser(js_identifier_start).parse("0foobar").is_err(), true);
	}

	#[test]
	pub fn test_js_identifier_part() {
		assert_eq!(parser(js_identifier_part).parse("").is_err(), true);
		assert_eq!(parser(js_identifier_part).parse(" ").is_err(), true);
		assert_eq!(
			parser(js_identifier_part).parse("_foobar").unwrap(),
			("_foobar".into(), "")
		);
		assert_eq!(
			parser(js_identifier_part).parse("$foobar").unwrap(),
			("$foobar".into(), "")
		);
		assert_eq!(
			parser(js_identifier_part).parse("afoobar").unwrap(),
			("afoobar".into(), "")
		);
		assert_eq!(
			parser(js_identifier_part).parse("0foobar").unwrap(),
			("0foobar".into(), "")
		);
	}

	#[test]
	pub fn test_js_boolean() {
		assert_eq!(parser(js_boolean).parse("").is_err(), true);
		assert_eq!(parser(js_boolean).parse(" ").is_err(), true);
		assert_eq!(
			parser(js_boolean).parse("true").unwrap(),
			(true.into(), "")
		);
		assert_eq!(
			parser(js_boolean).parse("false").unwrap(),
			(false.into(), "")
		);
		assert_eq!(parser(js_boolean).parse("a").is_err(), true);
	}

	#[test]
	pub fn test_js_number() {
		assert_eq!(parser(js_number).parse("").is_err(), true);
		assert_eq!(parser(js_number).parse(" ").is_err(), true);
		assert_eq!(parser(js_number).parse("1").unwrap(), (1f64.into(), ""));
		assert_eq!(parser(js_number).parse("+1").unwrap(), (1f64.into(), ""));
		assert_eq!(
			parser(js_number).parse("-1").unwrap(),
			((-1f64).into(), "")
		);
		assert_eq!(
			parser(js_number).parse("1.2").unwrap(),
			(1.2f64.into(), "")
		);
		assert_eq!(
			parser(js_number).parse("1e3").unwrap(),
			(1e3f64.into(), "")
		);
		assert_eq!(
			parser(js_number).parse("1.2e3").unwrap(),
			(1.2e3f64.into(), "")
		);
		assert_eq!(parser(js_number).parse("a").is_err(), true);
	}

	#[test]
	pub fn test_js_single_string_characters() {
		assert_eq!(
			parser(js_single_string_characters).parse("").is_err(),
			true
		);
		assert_eq!(
			parser(js_single_string_characters).parse(" ").is_err(),
			true
		);
		assert_eq!(
			parser(js_single_string_characters).parse(r#"''"#).unwrap(),
			("".into(), "")
		);
		assert_eq!(
			parser(js_single_string_characters).parse(r#"' '"#).unwrap(),
			(" ".into(), "")
		);
		assert_eq!(
			parser(js_single_string_characters)
				.parse(r#"'foo'"#)
				.unwrap(),
			("foo".into(), "")
		);
		assert_eq!(
			p(js_single_string_characters)
				.parse(r#"'foo\'bar'"#)
				.unwrap(),
			("foo'bar".into(), "")
		);
		assert_eq!(
			p(js_single_string_characters)
				.parse(r#"'foo"bar'"#)
				.unwrap(),
			("foo\"bar".into(), "")
		);
	}

	#[test]
	pub fn test_js_double_string_characters() {
		assert_eq!(
			parser(js_double_string_characters).parse("").is_err(),
			true
		);
		assert_eq!(
			parser(js_double_string_characters).parse(" ").is_err(),
			true
		);
		assert_eq!(
			parser(js_double_string_characters).parse(r#""""#).unwrap(),
			("".into(), "")
		);
		assert_eq!(
			parser(js_double_string_characters).parse(r#"" ""#).unwrap(),
			(" ".into(), "")
		);
		assert_eq!(
			parser(js_double_string_characters)
				.parse(r#""foo""#)
				.unwrap(),
			("foo".into(), "")
		);
		assert_eq!(
			p(js_double_string_characters)
				.parse(r#""foo\"bar""#)
				.unwrap(),
			("foo\"bar".into(), "")
		);
		assert_eq!(
			p(js_double_string_characters)
				.parse(r#""foo'bar""#)
				.unwrap(),
			("foo'bar".into(), "")
		);
	}

	#[test]
	pub fn test_js_single_line_comment() {
		assert_eq!(parser(js_single_line_comment).parse("").is_err(), true);
		assert_eq!(parser(js_single_line_comment).parse(" ").is_err(), true);
		assert_eq!(parser(js_single_line_comment).parse("//").is_err(), true);
		assert_eq!(
			parser(js_single_line_comment).parse("//\n").is_err(),
			false
		);
		assert_eq!(
			parser(js_single_line_comment)
				.parse("// hello world\n")
				.unwrap(),
			((), "")
		);
		assert_eq!(
			parser(js_single_line_comment)
				.parse("//// hello \t world\n")
				.unwrap(),
			((), "")
		);
	}

	#[test]
	pub fn test_js_multi_line_comment() {
		assert_eq!(parser(js_multi_line_comment).parse("").is_err(), true);
		assert_eq!(parser(js_multi_line_comment).parse(" ").is_err(), true);
		assert_eq!(parser(js_multi_line_comment).parse("/*").is_err(), true);
		assert_eq!(parser(js_multi_line_comment).parse("*/").is_err(), true);
		assert_eq!(parser(js_multi_line_comment).parse("/**/").is_err(), false);
		assert_eq!(
			parser(js_multi_line_comment)
				.parse("/* hello world */")
				.is_err(),
			false
		);
		assert_eq!(
			parser(js_multi_line_comment)
				.parse("/* hello \n world */")
				.is_err(),
			false
		);
		assert_eq!(
			parser(js_multi_line_comment)
				.parse("/*** hello \n world ***/")
				.is_err(),
			false
		);
	}

	#[test]
	pub fn test_js_whitespace() {
		assert_eq!(parser(js_whitespace).parse("").unwrap(), ((), ""));
		assert_eq!(parser(js_whitespace).parse(" ").unwrap(), ((), ""));
		assert_eq!(parser(js_whitespace).parse("a").unwrap(), ((), "a"));
		assert_eq!(
			parser(js_whitespace).parse("   foo bar").unwrap(),
			((), "foo bar")
		);
		assert_eq!(
			p(js_whitespace)
				.parse("  /*** hello \n world ***/  foo bar")
				.unwrap(),
			((), "foo bar")
		);
		assert_eq!(
			p(js_whitespace)
				.parse("  //// hello \t world  \n  foo bar")
				.unwrap(),
			((), "foo bar")
		);
	}
}
