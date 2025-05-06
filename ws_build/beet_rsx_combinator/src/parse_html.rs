use combine::ParseResult;
use combine::Parser;
use combine::Stream;
use combine::char::string;
use combine::choice;
use combine::combinator::between;
use combine::combinator::none_of;
use combine::combinator::token;
use combine::combinator::r#try;
use combine::combinator::value;
use combine::skip_many;

// TODO doctype


pub fn html_comment<I>(input: I) -> ParseResult<(), I>
where
	I: Stream<Item = char>,
{
	between(
		string("<!--"),
		string("-->"),
		// a skip_until but for this early version of combine
		skip_many(choice!(
			r#try(
				(token('-'), token('-'), none_of(">".chars())).with(value(()))
			),
			r#try((token('-'), none_of("-".chars())).with(value(()))),
			none_of("-".chars()).with(value(()))
		)),
	)
	.parse_stream(input)
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use combine::Parser;
	use combine::combinator::parser;


	#[test]
	fn test_html_comment() {
		assert_eq!(
			parser(html_comment).parse("<!--a - > comment->-->").unwrap(),
			((), "")
		);
	}
}
