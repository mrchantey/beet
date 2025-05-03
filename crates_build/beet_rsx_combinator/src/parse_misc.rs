use combine::ParseResult;
use combine::Parser;
use combine::Stream;
use combine::char::digit;
use combine::char::spaces;
use combine::choice;
use combine::combinator::many1;
use combine::combinator::one_of;
use combine::combinator::optional;
use combine::combinator::parser;
use combine::combinator::token;
use combine::combinator::value;

pub fn open_tag<I>(input: I) -> ParseResult<(), I>
where
	I: Stream<Item = char>,
{
	token('<').with(value(())).parse_stream(input)
}

pub fn close_tag<I>(input: I) -> ParseResult<(), I>
where
	I: Stream<Item = char>,
{
	token('>').with(value(())).parse_stream(input)
}

pub fn closing_element_open_tag<I>(input: I) -> ParseResult<(), I>
where
	I: Stream<Item = char>,
{
	(token('<').skip(spaces()), token('/'))
		.with(value(()))
		.parse_stream(input)
}

pub fn self_closing_element_close_tag<I>(input: I) -> ParseResult<(), I>
where
	I: Stream<Item = char>,
{
	(token('/').skip(spaces()), token('>'))
		.with(value(()))
		.parse_stream(input)
}

pub fn sign<I>(input: I) -> ParseResult<char, I>
where
	I: Stream<Item = char>,
{
	choice!(token('-'), token('+')).parse_stream(input)
}

pub fn frac<I>(input: I) -> ParseResult<i64, I>
where
	I: Stream<Item = char>,
{
	token('.')
		.with(many1(digit()))
		.map(|frac: String| frac.parse::<i64>().unwrap())
		.parse_stream(input)
}

pub fn exp<I>(input: I) -> ParseResult<i64, I>
where
	I: Stream<Item = char>,
{
	choice!(token('e'), token('E'))
		.with(parser(integer))
		.parse_stream(input)
}

pub fn integer<I>(input: I) -> ParseResult<i64, I>
where
	I: Stream<Item = char>,
{
	(optional(parser(sign)), many1(digit()))
		.map(|(sign, digits): (Option<_>, String)| {
			format!("{}{}", sign.unwrap_or('+'), digits)
				.parse::<i64>()
				.unwrap()
		})
		.parse_stream(input)
}

pub fn float_simple<I>(input: I) -> ParseResult<f64, I>
where
	I: Stream<Item = char>,
{
	(parser(integer), parser(frac))
		.map(|(int, frac)| format!("{}.{}", int, frac).parse::<f64>().unwrap())
		.parse_stream(input)
}

pub fn float_exp<I>(input: I) -> ParseResult<f64, I>
where
	I: Stream<Item = char>,
{
	(parser(integer), optional(parser(frac)), parser(exp))
		.map(|(int, frac, exp)| {
			format!("{}.{}e{}", int, frac.unwrap_or(0), exp)
				.parse::<f64>()
				.unwrap()
		})
		.parse_stream(input)
}

pub fn identifier_non_alpha_numeric<I>(input: I) -> ParseResult<char, I>
where
	I: Stream<Item = char>,
{
	choice!(token('_'), token('$')).parse_stream(input)
}

pub fn escaped_character<I>(input: I) -> ParseResult<char, I>
where
	I: Stream<Item = char>,
{
	token('\\')
		.with(one_of(r#"'"\nrtbfv0"#.chars()))
		.map(|c| match c {
			'\'' => '\'',
			'"' => '"',
			'\\' => '\\',
			'n' => '\n',
			'r' => '\r',
			't' => '\t',
			'b' => '\u{0008}',
			'f' => '\u{000c}',
			'v' => '\u{2b7f}',
			'0' => '\u{0000}',
			_ => panic!("Unhandled escape character"),
		})
		.parse_stream(input)
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	pub fn test_open_tag() {
		assert_eq!(parser(open_tag).parse("").is_err(), true);
		assert_eq!(parser(open_tag).parse("foo").is_err(), true);
		assert_eq!(parser(open_tag).parse("<").unwrap(), ((), ""));
		assert_eq!(parser(open_tag).parse("foo<").is_err(), true);
	}

	#[test]
	pub fn test_close_tag() {
		assert_eq!(parser(close_tag).parse("").is_err(), true);
		assert_eq!(parser(close_tag).parse("foo").is_err(), true);
		assert_eq!(parser(close_tag).parse(">").unwrap(), ((), ""));
		assert_eq!(parser(close_tag).parse("foo>").is_err(), true);
	}

	#[test]
	pub fn test_closing_element_open_tag() {
		assert_eq!(parser(closing_element_open_tag).parse("").is_err(), true);
		assert_eq!(
			parser(closing_element_open_tag).parse("foo").is_err(),
			true
		);
		assert_eq!(
			parser(closing_element_open_tag).parse("</").unwrap(),
			((), "")
		);
		assert_eq!(
			parser(closing_element_open_tag).parse("foo</").is_err(),
			true
		);
	}

	#[test]
	pub fn test_self_closing_element_close_tag() {
		assert_eq!(
			parser(self_closing_element_close_tag).parse("").is_err(),
			true
		);
		assert_eq!(
			parser(self_closing_element_close_tag).parse("foo").is_err(),
			true
		);
		assert_eq!(
			parser(self_closing_element_close_tag).parse("/>").unwrap(),
			((), "")
		);
		assert_eq!(
			parser(self_closing_element_close_tag)
				.parse("foo/>")
				.is_err(),
			true
		);
	}

	#[test]
	pub fn test_sign() {
		assert_eq!(parser(sign).parse("").is_err(), true);
		assert_eq!(parser(sign).parse(" ").is_err(), true);
		assert_eq!(parser(sign).parse("+").unwrap(), ('+', ""));
		assert_eq!(parser(sign).parse("-").unwrap(), ('-', ""));
		assert_eq!(parser(sign).parse("a").is_err(), true);
	}

	#[test]
	pub fn test_frac() {
		assert_eq!(parser(frac).parse("").is_err(), true);
		assert_eq!(parser(frac).parse(" ").is_err(), true);
		assert_eq!(parser(frac).parse("1234").is_err(), true);
		assert_eq!(parser(frac).parse(".1234").unwrap(), (1234, ""));
		assert_eq!(parser(frac).parse("a").is_err(), true);
	}

	#[test]
	pub fn test_exp() {
		assert_eq!(parser(exp).parse("").is_err(), true);
		assert_eq!(parser(exp).parse(" ").is_err(), true);
		assert_eq!(parser(exp).parse("1234").is_err(), true);
		assert_eq!(parser(exp).parse("e1234").unwrap(), (1234, ""));
		assert_eq!(parser(exp).parse("e+1234").unwrap(), (1234, ""));
		assert_eq!(parser(exp).parse("e-1234").unwrap(), (-1234, ""));
		assert_eq!(parser(exp).parse("a").is_err(), true);
	}

	#[test]
	pub fn test_integer() {
		assert_eq!(parser(integer).parse("").is_err(), true);
		assert_eq!(parser(integer).parse(" ").is_err(), true);
		assert_eq!(parser(integer).parse("1").unwrap(), (1i64, ""));
		assert_eq!(parser(integer).parse("1.2").unwrap(), (1i64, ".2"));
		assert_eq!(parser(integer).parse("1e3").unwrap(), (1i64, "e3"));
		assert_eq!(parser(integer).parse("1.2e3").unwrap(), (1i64, ".2e3"));
		assert_eq!(parser(integer).parse("+1").unwrap(), (1i64, ""));
		assert_eq!(parser(integer).parse("-1").unwrap(), (-1i64, ""));
	}

	#[test]
	pub fn test_float_simple() {
		assert_eq!(parser(float_simple).parse("").is_err(), true);
		assert_eq!(parser(float_simple).parse(" ").is_err(), true);
		assert_eq!(parser(float_simple).parse("1").is_err(), true);
		assert_eq!(parser(float_simple).parse("1.2").unwrap(), (1.2f64, ""));
		assert_eq!(parser(float_simple).parse("1e3").is_err(), true);
		assert_eq!(
			parser(float_simple).parse("1.2e3").unwrap(),
			(1.2f64, "e3")
		);
		assert_eq!(parser(float_simple).parse("+1.2").unwrap(), (1.2f64, ""));
		assert_eq!(parser(float_simple).parse("-1.2").unwrap(), (-1.2f64, ""));
	}

	#[test]
	pub fn test_float_exp() {
		assert_eq!(parser(float_exp).parse("").is_err(), true);
		assert_eq!(parser(float_exp).parse(" ").is_err(), true);
		assert_eq!(parser(float_exp).parse("1").is_err(), true);
		assert_eq!(parser(float_exp).parse("1.2").is_err(), true);
		assert_eq!(parser(float_exp).parse("1e3").unwrap(), (1e3f64, ""));
		assert_eq!(parser(float_exp).parse("1.2e3").unwrap(), (1.2e3f64, ""));
		assert_eq!(parser(float_exp).parse("+1.2e3").unwrap(), (1.2e3f64, ""));
		assert_eq!(parser(float_exp).parse("-1.2e3").unwrap(), (-1.2e3f64, ""));
	}

	#[test]
	pub fn test_identifier_non_alpha_numeric() {
		assert_eq!(
			parser(identifier_non_alpha_numeric).parse("").is_err(),
			true
		);
		assert_eq!(
			parser(identifier_non_alpha_numeric).parse(" ").is_err(),
			true
		);
		assert_eq!(
			parser(identifier_non_alpha_numeric).parse("_").unwrap(),
			('_', "")
		);
		assert_eq!(
			parser(identifier_non_alpha_numeric).parse("$").unwrap(),
			('$', "")
		);
		assert_eq!(
			parser(identifier_non_alpha_numeric).parse("a").is_err(),
			true
		);
		assert_eq!(
			parser(identifier_non_alpha_numeric).parse("0").is_err(),
			true
		);
	}

	#[test]
	pub fn test_escaped_character() {
		assert_eq!(parser(escaped_character).parse("").is_err(), true);
		assert_eq!(parser(escaped_character).parse(" ").is_err(), true);
		assert_eq!(
			parser(escaped_character).parse(r#"\""#).unwrap(),
			('"', "")
		);
		assert_eq!(
			parser(escaped_character).parse(r#"\'"#).unwrap(),
			('\'', "")
		);
		assert_eq!(parser(escaped_character).parse(r#"""#).is_err(), true);
		assert_eq!(parser(escaped_character).parse(r#"'"#).is_err(), true);
	}
}
