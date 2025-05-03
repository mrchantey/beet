use crate::prelude::*;
use combine::ParseResult;
use combine::Parser;
use combine::Stream;
use combine::choice;
use combine::combinator::between;
use combine::combinator::many;
use combine::combinator::none_of;
use combine::combinator::parser;
use combine::combinator::token;
use combine::combinator::r#try;

pub fn rs_char<I>(input: I) -> ParseResult<RSChar, I>
where
	I: Stream<Item = char>,
{
	between(
		token('\''),
		token('\''),
		choice!(r#try(parser(escaped_character)), none_of("'".chars())),
	)
	.map(RSChar)
	.parse_stream(input)
}

pub fn rs_string<I>(input: I) -> ParseResult<RSString, I>
where
	I: Stream<Item = char>,
{
	between(
		token('"'),
		token('"'),
		many(choice!(
			r#try(parser(escaped_character)),
			none_of("\"".chars())
		)),
	)
	.map(RSString)
	.parse_stream(input)
}

pub fn rs_comment<I>(input: I) -> ParseResult<(), I>
where
	I: Stream<Item = char>,
{
	parser(js_comment).parse_stream(input)
}

pub fn rs_whitespace<I>(input: I) -> ParseResult<(), I>
where
	I: Stream<Item = char>,
{
	parser(js_whitespace).parse_stream(input)
}
