use crate::prelude::*;
use combine::ParseError;
use combine::Parser;
use combine::combinator::parser;



pub struct CombinatorParser;

impl CombinatorParser {
	/// Parse a string into a [`RsxChildren`] structure, allowing for the
	/// following top-level constructs:
	/// - text 							`"hello"`
	/// - blocks 						`{let foo = "bar"; foo}`
	/// - single elements 	`<br/>`
	/// - multiple elements `<br/><br/>`
	/// - mixed 						`"hello"<br/>{let foo = "bar"; foo}`
	pub fn parse(s: &str) -> Result<(RsxChildren, &str), ParseError<&str>> {
		parser(rsx_children).skip(parser(js_whitespace)).parse(s)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	pub fn top_level_expression() {
		let source = "let a = <br/>; a";
		let (ast, _remaining) = CombinatorParser::parse(source).unwrap();
		expect(ast.to_html()).to_be("let a =<br/>; a");
	}
	#[test]
	pub fn style_tags() {
		let source = "<style>body {padding: 1em}</style>";
		let (ast, _remaining) = CombinatorParser::parse(source).unwrap();
		expect(ast.to_html()).to_be("<style>body{padding: 1em}</style>");
	}

	#[test]
	pub fn simple() {
		let source = "<foo>Hello world!</foo>";
		let (ast, _) = CombinatorParser::parse(source).unwrap();

		expect(ast.to_html()).to_be("<foo>Hello world!</foo>");
	}

	#[test]
	pub fn expression_attributes() {
		let source =
			"<div hidden style={stylesheet.get(\".foo\")}>Hello world!</div>";
		let (ast, _) = CombinatorParser::parse(source).unwrap();
		expect(ast.to_html()).to_be(
			"<div hidden style={stylesheet.get(\".foo\")}>Hello world!</div>",
		);
	}

	#[test]
	pub fn dashed_node_tags() {
		let source = "<x-foo-bar>Hello world!</x-foo-bar>";
		let (ast, _) = CombinatorParser::parse(source).unwrap();
		expect(ast.to_html()).to_be("<x-foo-bar>Hello world!</x-foo-bar>");
	}

	#[test]
	pub fn empty_fragment() {
		let source = "<></>";
		let (ast, _) = CombinatorParser::parse(source).unwrap();
		expect(ast.to_html()).to_be("");
	}

	#[test]
	pub fn fragment_with_single_element() {
		let source = "<><div>Hello</div></>";
		let (ast, _) = CombinatorParser::parse(source).unwrap();
		expect(ast.to_html()).to_be("<div>Hello</div>");
	}

	#[test]
	pub fn fragment_with_multiple_elements() {
		let source = "<><div>Hello</div><span>World</span></>";
		let (ast, _) = CombinatorParser::parse(source).unwrap();
		expect(ast.to_html()).to_be("<div>Hello</div><span>World</span>");
	}

	#[test]
	pub fn fragment_with_text_and_elements() {
		let source = "<>Text before<div>content</div>Text after</>";
		let (ast, _) = CombinatorParser::parse(source).unwrap();
		expect(ast.to_html()).to_be("Text before<div>content</div>Text after");
	}

	#[test]
	pub fn nested_fragments() {
		let source = "<><>Inner fragment</><div>Element</div></>";
		let (ast, _) = CombinatorParser::parse(source).unwrap();
		expect(ast.to_html()).to_be("Inner fragment<div>Element</div>");
	}
	#[test]
	pub fn multiple_root_elements() {
		let source = "<br/><br/>";
		let (ast, _) = CombinatorParser::parse(source).unwrap();
		expect(ast.to_html()).to_be("<br/><br/>");
	}
}
