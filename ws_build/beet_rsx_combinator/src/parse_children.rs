use crate::prelude::*;
use combine::ParseResult;
use combine::Parser;
use combine::Stream;
use combine::char::spaces;
use combine::choice;
use combine::combinator::look_ahead;
use combine::combinator::many1;
use combine::combinator::none_of;
use combine::combinator::one_of;
use combine::combinator::optional;
use combine::combinator::parser;
use combine::combinator::r#try;

pub fn rsx_children<I>(input: I) -> ParseResult<RsxChildren, I>
where
	I: Stream<Item = char>,
{
	many1(parser(rsx_child).skip(parser(js_whitespace))).parse_stream(input)
}

pub fn rsx_child<I>(input: I) -> ParseResult<RsxChild, I>
where
	I: Stream<Item = char>,
{
	choice!(
		r#try(parser(rsx_code_block).map(RsxChild::CodeBlock)),
		r#try(parser(rsx_element).map(RsxChild::Element)),
		parser(rsx_text).map(RsxChild::Text)
	)
	.parse_stream(input)
}

pub fn rsx_text<I>(input: I) -> ParseResult<RsxText, I>
where
	I: Stream<Item = char>,
{
	many1(parser(rsx_text_character)).parse_stream(input)
}

pub fn rsx_text_character<I>(input: I) -> ParseResult<RsxTextCharacter, I>
where
	I: Stream<Item = char>,
{
	let invalid = "{}<>";
	none_of(invalid.chars())
		.skip(optional(r#try(
			spaces().with(look_ahead(one_of(invalid.chars()))),
		)))
		.map(RsxTextCharacter)
		.parse_stream(input)
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::to_html::ToHtml;

	#[test]
	pub fn test_rsx_children_to_html() {
		let value = parser(rsx_children)
			.parse(
				r#"foo
                   foo bar baz
                   123
                   <foo/>
                   {<foo/>}
                   {{1+{2+{3}}}}
                "#,
			)
			.unwrap()
			.0;
		// whitespace is not preserved between elements and blocks
		let expected = r#"foo
                   foo bar baz
                   123<foo/>{<foo/>}{{1+{2+{3}}}}"#;

		assert_eq!(value.to_html(), expected);
	}

	#[test]
	pub fn test_rsx_children() {
		assert_eq!(parser(rsx_children).parse("").is_err(), true);
		assert_eq!(
			parser(rsx_children).parse(" ").unwrap(),
			(RsxChildren::from(vec![" ".into()]), "")
		);
		assert_eq!(
			parser(rsx_children).parse("foo").unwrap(),
			(RsxChildren::from(vec!["foo".into()]), "")
		);
		assert_eq!(
			parser(rsx_children).parse("foo!").unwrap(),
			(RsxChildren::from(vec!["foo!".into()]), "")
		);
		assert_eq!(
			parser(rsx_children).parse("foo bar baz!").unwrap(),
			(RsxChildren::from(vec!["foo bar baz!".into()]), "")
		);
		assert_eq!(
			parser(rsx_children)
				.parse("\"foo\" \"bar\" \"baz\"")
				.unwrap(),
			(
				RsxChildren::from(vec!["\"foo\" \"bar\" \"baz\"".into()]),
				""
			)
		);
		assert_eq!(
			parser(rsx_children)
				.parse("\"foo\"\n\"bar\"\n\"baz\"")
				.unwrap(),
			(
				RsxChildren::from(vec!["\"foo\"\n\"bar\"\n\"baz\"".into()]),
				""
			)
		);
	}

	#[test]
	pub fn test_rsx_child() {
		assert_eq!(parser(rsx_child).parse("").is_err(), true);
		assert_eq!(parser(rsx_child).parse(" ").unwrap(), (" ".into(), ""));
		assert_eq!(parser(rsx_child).parse("foo").unwrap(), ("foo".into(), ""));
		assert_eq!(
			parser(rsx_child).parse("\"foo\" \"bar\" \"baz\"").unwrap(),
			("\"foo\" \"bar\" \"baz\"".into(), "")
		);
		assert_eq!(
			parser(rsx_child)
				.parse("\"foo\"\n\"bar\"\n\"baz\"")
				.unwrap(),
			("\"foo\"\n\"bar\"\n\"baz\"".into(), "")
		);
	}

	#[test]
	pub fn test_rsx_text() {
		assert_eq!(parser(rsx_text).parse("").is_err(), true);
		assert_eq!(
			parser(rsx_text).parse("foo < bar").unwrap(),
			("foo".into(), "< bar")
		);
		assert_eq!(
			parser(rsx_text).parse("foo > bar").unwrap(),
			("foo".into(), "> bar")
		);
		assert_eq!(
			parser(rsx_text).parse("foo bar baz").unwrap(),
			("foo bar baz".into(), "")
		);
		assert_eq!(
			parser(rsx_text).parse("foo { bar } baz").unwrap(),
			("foo".into(), "{ bar } baz")
		);
	}

	#[test]
	pub fn test_rsx_text_character() {
		assert_eq!(parser(rsx_text_character).parse("").is_err(), true);
		assert_eq!(parser(rsx_text_character).parse("{").is_err(), true);
		assert_eq!(parser(rsx_text_character).parse("}").is_err(), true);
		assert_eq!(parser(rsx_text_character).parse("<").is_err(), true);
		assert_eq!(parser(rsx_text_character).parse(">").is_err(), true);
		assert_eq!(
			parser(rsx_text_character).parse(" ").unwrap(),
			(' '.into(), "")
		);
		assert_eq!(
			parser(rsx_text_character).parse("_").unwrap(),
			('_'.into(), "")
		);
		assert_eq!(
			parser(rsx_text_character).parse("$").unwrap(),
			('$'.into(), "")
		);
		assert_eq!(
			parser(rsx_text_character).parse("a").unwrap(),
			('a'.into(), "")
		);
		assert_eq!(
			parser(rsx_text_character).parse("0").unwrap(),
			('0'.into(), "")
		);
	}
}
