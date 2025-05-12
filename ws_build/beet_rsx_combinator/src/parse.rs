use crate::prelude::*;
use combine::ParseError;
use combine::ParseResult;
use combine::Parser;
use combine::Stream;
use combine::combinator::parser;
use combine::many;
use combine::optional;



/// Parse a string of code containing rsx, the output will be
/// an expression enclosed in `{`block quotes`}`
pub fn parse(s: &str) -> Result<(RsxParsedExpression, &str), ParseError<&str>> {
	let (items, remainder) = parser(rsx_code_ignoring_ws).parse(s)?;
	Ok((items.into_iter().collect(), remainder))
}

fn rsx_code_ignoring_ws<I>(input: I) -> ParseResult<Vec<RsxRawCodeFragment>, I>
where
	I: Stream<Item = char>,
{
	optional(parser(js_whitespace))
		.with(many(parser(rsx_code_block_fragment)).skip(parser(js_whitespace)))
		.parse_stream(input)
}
