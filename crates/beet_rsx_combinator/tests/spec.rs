#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_rsx_combinator::prelude::*;
use sweet::prelude::*;

#[test]
pub fn top_level_expression() {
	let source = "let a = <br/>; a";
	let (ast, _remaining) = parse(source).unwrap();
	expect(ast.to_html()).to_be("{let a = <br/>; a}");
}
#[test]
pub fn style_tags() {
	let source = "<style>body {padding: 1em}</style>";
	let (ast, _remaining) = parse(source).unwrap();
	expect(ast.to_html()).to_be("{<style>body{padding: 1em}</style>}");
}

#[test]
pub fn test_to_html_1() {
	let source = "<foo>Hello world!</foo>";
	let (ast, _) = parse(source).unwrap();

	expect(ast.to_html()).to_be("{<foo>Hello world!</foo>}");
}

#[test]
pub fn test_to_html_2() {
	let source =
		"<div hidden style={stylesheet.get(\".foo\")}>Hello world!</div>";
	let (ast, _) = parse(source).unwrap();
	expect(ast.to_html()).to_be(
		"{<div hidden style={stylesheet.get(\".foo\")}>Hello world!</div>}",
	);
}

#[test]
pub fn test_to_html_3() {
	let source = "<x-foo-bar>Hello world!</x-foo-bar>";
	let (ast, _) = parse(source).unwrap();
	expect(ast.to_html()).to_be("{<x-foo-bar>Hello world!</x-foo-bar>}");
}

#[test]
pub fn test_fragment_empty() {
	let source = "<></>";
	let (ast, _) = parse(source).unwrap();
	expect(ast.to_html()).to_be("{}");
}

#[test]
pub fn test_fragment_with_single_element() {
	let source = "<><div>Hello</div></>";
	let (ast, _) = parse(source).unwrap();
	expect(ast.to_html()).to_be("{<div>Hello</div>}");
}

#[test]
pub fn test_fragment_with_multiple_elements() {
	let source = "<><div>Hello</div><span>World</span></>";
	let (ast, _) = parse(source).unwrap();
	expect(ast.to_html()).to_be("{<div>Hello</div><span>World</span>}");
}

#[test]
pub fn test_fragment_with_text_and_elements() {
	let source = "<>Text before<div>content</div>Text after</>";
	let (ast, _) = parse(source).unwrap();
	expect(ast.to_html()).to_be("{Text before<div>content</div>Text after}");
}

#[test]
pub fn test_nested_fragments() {
	let source = "<><>Inner fragment</><div>Element</div></>";
	let (ast, _) = parse(source).unwrap();
	expect(ast.to_html()).to_be("{Inner fragment<div>Element</div>}");
}
