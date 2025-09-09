use crate::prelude::*;
use combine::ParseError;
use combine::Parser;
use combine::combinator::parser;



pub struct CombinatorParser;

impl CombinatorParser {
	/// Parse a string into a [`RsxChildren`] structure as if the string is the inner
	/// of a fragment, allowing for the following top-level constructs:
	/// - text 							`"hello"`
	/// - blocks 						`{let foo = "bar"; foo}`
	/// - single elements 	`<br/>`
	/// - multiple elements `<br/><br/>`
	/// - mixed 						`"hello"<br/>{let foo = "bar"; foo}`
	///
	/// ## Example
	///
	/// ```
	/// # use beet_rsx_combinator::prelude::*;
	///
	/// ```
	///
	pub fn parse<'a>(
		tokens: &'a str,
	) -> Result<RsxChildren, CombinatorParserError<'a>> {
		if tokens.is_empty() {
			return Ok(RsxChildren::default());
		}
		let (children, remaining) = parser(rsx_children)
			.skip(parser(js_whitespace))
			.parse(tokens)?;
		if !remaining.trim().is_empty() {
			return Err(CombinatorParserError::RemainingText(remaining));
		}

		Ok(children)
	}
}


#[derive(Debug, thiserror::Error)]
pub enum CombinatorParserError<'a> {
	#[error("ParseError({0})")]
	ParseError(ParseError<&'a str>),
	#[error("RemainingText({0})")]
	RemainingText(&'a str),
}

impl<'a> From<ParseError<&'a str>> for CombinatorParserError<'a> {
	fn from(err: ParseError<&'a str>) -> Self {
		CombinatorParserError::ParseError(err)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	pub fn empty() {
		CombinatorParser::parse("").unwrap().to_html().xpect_eq("");
	}
	#[test]
	pub fn top_level_expression() {
		CombinatorParser::parse("let a = <br/>; a")
			.unwrap()
			.to_html()
			.xpect_eq("let a =<br/>; a");
	}

	#[test]
	pub fn style_tags() {
		CombinatorParser::parse("<style>body {padding: 1em}</style>")
			.unwrap()
			.to_html()
			.xpect_eq("<style>body{padding: 1em}</style>");
	}

	#[test]
	pub fn simple() {
		CombinatorParser::parse("<foo>Hello world!</foo>")
			.unwrap()
			.to_html()
			.xpect_eq("<foo>Hello world!</foo>");
	}

	#[test]
	pub fn expression_attributes() {
		CombinatorParser::parse(
			"<div hidden style={stylesheet.get(\".foo\")}>Hello world!</div>",
		)
		.unwrap()
		.to_html()
		.xpect_eq(
			"<div hidden style={stylesheet.get(\".foo\")}>Hello world!</div>",
		);
	}

	#[test]
	pub fn dashed_node_tags() {
		CombinatorParser::parse("<x-foo-bar>Hello world!</x-foo-bar>")
			.unwrap()
			.to_html()
			.xpect_eq("<x-foo-bar>Hello world!</x-foo-bar>");
	}

	#[test]
	pub fn empty_fragment() {
		CombinatorParser::parse("<></>")
			.unwrap()
			.to_html()
			.xpect_eq("");
	}

	#[test]
	pub fn fragment_with_single_element() {
		CombinatorParser::parse("<><div>Hello</div></>")
			.unwrap()
			.to_html()
			.xpect_eq("<div>Hello</div>");
	}

	#[test]
	pub fn fragment_with_multiple_elements() {
		CombinatorParser::parse("<><div>Hello</div><span>World</span></>")
			.unwrap()
			.to_html()
			.xpect_eq("<div>Hello</div><span>World</span>");
	}

	#[test]
	pub fn fragment_with_text_and_elements() {
		CombinatorParser::parse("<>Text before<div>content</div>Text after</>")
			.unwrap()
			.to_html()
			.xpect_eq("Text before<div>content</div>Text after");
	}

	#[test]
	pub fn nested_fragments() {
		CombinatorParser::parse("<><>Inner fragment</><div>Element</div></>")
			.unwrap()
			.to_html()
			.xpect_eq("Inner fragment<div>Element</div>");
	}

	#[test]
	#[ignore = "TODO"]
	pub fn preserves_whitespace() {
		CombinatorParser::parse("<p>i am <strong>very</strong> cool</p>")
			.unwrap()
			.to_html()
			.xpect_eq("<p>i am <strong>very</strong> cool</p>");
	}

	#[test]
	pub fn multiple_root_elements() {
		CombinatorParser::parse("<br/><br/>")
			.unwrap()
			.to_html()
			.xpect_eq("<br/><br/>");
	}
}
