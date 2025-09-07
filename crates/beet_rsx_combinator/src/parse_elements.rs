use crate::prelude::*;
use combine::ParseResult;
use combine::Parser;
use combine::Stream;
use combine::choice;
use combine::combinator::env_parser;
use combine::combinator::look_ahead;
use combine::combinator::optional;
use combine::combinator::parser;
use combine::combinator::sep_by1;
use combine::combinator::token;
use combine::combinator::tokens;
use combine::combinator::r#try;
use itertools::Itertools;

pub fn rsx_element<I>(input: I) -> ParseResult<RsxElement, I>
where
	I: Stream<Item = char>,
{
	choice!(
		r#try(parser(rsx_self_closing_element).map(RsxElement::SelfClosing)),
		r#try(parser(rsx_fragment).map(RsxElement::Fragment)),
		parser(rsx_normal_element).map(RsxElement::Normal)
	)
	.parse_stream(input)
}

pub fn rsx_element_open<I>(input: I) -> ParseResult<RsxElementName, I>
where
	I: Stream<Item = char>,
{
	parser(open_tag)
		.skip(parser(js_whitespace))
		.with(parser(rsx_element_name))
		.parse_stream(input)
}

pub fn rsx_self_closing_element<I>(
	input: I,
) -> ParseResult<RsxSelfClosingElement, I>
where
	I: Stream<Item = char>,
{
	(
		parser(rsx_element_open).skip(parser(js_whitespace)),
		optional(parser(rsx_attributes).skip(parser(js_whitespace))),
		parser(self_closing_element_close_tag),
	)
		.map(|(n, a, _)| RsxSelfClosingElement(n, a.into()))
		.parse_stream(input)
}

pub fn rsx_normal_element<I>(input: I) -> ParseResult<RsxNormalElement, I>
where
	I: Stream<Item = char>,
{
	look_ahead(parser(rsx_element_open))
		.parse_stream(input)
		.and_then(|(name, consumed)| {
			(
				parser(rsx_opening_element).skip(parser(js_whitespace)),
				optional(parser(rsx_children).skip(parser(js_whitespace))),
				env_parser(&name, rsx_closing_element),
			)
				.map(|(RsxOpeningElement(n, a), c, _)| {
					RsxNormalElement(n, a, c.into())
				})
				.parse_stream(consumed.into_inner())
		})
}

pub fn rsx_fragment<I>(input: I) -> ParseResult<RsxFragment, I>
where
	I: Stream<Item = char>,
{
	(
		parser(fragment_open_tag).skip(parser(js_whitespace)),
		optional(parser(rsx_children).skip(parser(js_whitespace))),
		parser(fragment_close_tag),
	)
		.map(|(_, children, _)| RsxFragment(children.into()))
		.parse_stream(input)
}

pub fn rsx_opening_element<I>(input: I) -> ParseResult<RsxOpeningElement, I>
where
	I: Stream<Item = char>,
{
	(
		parser(rsx_element_open).skip(parser(js_whitespace)),
		optional(parser(rsx_attributes).skip(parser(js_whitespace))),
		parser(close_tag),
	)
		.map(|(n, a, _)| RsxOpeningElement(n, a.into()))
		.parse_stream(input)
}

pub fn rsx_closing_element<I>(
	name: &RsxElementName,
	input: I,
) -> ParseResult<RsxClosingElement<'_>, I>
where
	I: Stream<Item = char>,
{
	let chars = name.to_string();
	let chars_ws = chars.split('-').join(" - ");
	let expected = chars.clone().into();
	let expected_ws = chars_ws.clone().into();
	let cmp_ignore_ws =
		|l: char, r: char| l == r || l.is_whitespace() && r.is_whitespace();
	(
		parser(closing_element_open_tag).skip(parser(js_whitespace)),
		choice!(
			r#try(
				tokens(&cmp_ignore_ws, expected_ws, chars_ws.chars())
					.skip(parser(js_whitespace))
			),
			tokens(&cmp_ignore_ws, expected, chars.chars())
				.skip(parser(js_whitespace))
		),
		parser(close_tag),
	)
		.map(|_| RsxClosingElement(name))
		.parse_stream(input)
}

pub fn rsx_element_name<I>(input: I) -> ParseResult<RsxElementName, I>
where
	I: Stream<Item = char>,
{
	choice!(
		r#try(
			parser(rsx_member_expression).map(RsxElementName::MemberExpression)
		),
		r#try(
			parser(rsx_namespaced_name)
				.map(|(ns, n)| RsxElementName::NamedspacedName(ns, n))
		),
		parser(rsx_identifier).map(RsxElementName::Name)
	)
	.parse_stream(input)
}

pub fn rsx_identifier_simple<I>(input: I) -> ParseResult<RsxIdentifier, I>
where
	I: Stream<Item = char>,
{
	(
		r#try(parser(js_identifier_start)),
		optional(parser(js_identifier_part)),
	)
		.map(|(s, p)| {
			p.map(|p| format!("{}{}", s.0, p.0))
				.unwrap_or_else(|| format!("{}", s.0))
		})
		.map(RsxIdentifier)
		.parse_stream(input)
}

pub fn rsx_identifier<I>(input: I) -> ParseResult<RsxIdentifier, I>
where
	I: Stream<Item = char>,
{
	sep_by1(
		parser(rsx_identifier_simple).skip(parser(js_whitespace)),
		token('-').skip(parser(js_whitespace)),
	)
	.parse_stream(input)
}

pub fn rsx_namespaced_name<I>(
	input: I,
) -> ParseResult<(RsxIdentifier, RsxIdentifier), I>
where
	I: Stream<Item = char>,
{
	(parser(rsx_identifier), token(':'), parser(rsx_identifier))
		.map(|(l, _, r)| (l, r))
		.parse_stream(input)
}

pub fn rsx_member_expression<I>(
	input: I,
) -> ParseResult<Box<[RsxIdentifier]>, I>
where
	I: Stream<Item = char>,
{
	(
		parser(rsx_identifier),
		token('.'),
		sep_by1(parser(rsx_identifier), token('.')),
	)
		.map(|(i, _, mut v): (_, _, Vec<_>)| {
			v.insert(0, i);
			v.into_boxed_slice()
		})
		.parse_stream(input)
}

#[cfg(test)]
mod test {
	use super::env_parser as env_p;
	use super::parser as p;
	use super::*;

	#[test]
	pub fn test_rsx_element_tokenize() {
		let value = parser(rsx_element)
			.parse(
				r#"<root>
                     <foo/>
                     <foo.member.bar/>
                     <foo-bar/>
                     <foo - bar/>
                     <foo-bar.member/>
                     <foo - bar.member/>
                     <foo-bar.member.bar-baz/>
                     <foo - bar.member.bar - baz/>
                     <foo:bar/>
                     <foo-a:bar-b/>
                     <foo-a:bar-b/>
                     <foo></foo>
                     <x - foo - bar></x - foo - bar>
                   </root>
                "#,
			)
			.unwrap()
			.0;

		let expected = r#"<root><foo/><foo.member.bar/><foo-bar/><foo-bar/><foo-bar.member/><foo-bar.member/><foo-bar.member.bar-baz/><foo-bar.member.bar-baz/><foo:bar/><foo-a:bar-b/><foo-a:bar-b/><foo></foo><x-foo-bar></x-foo-bar></root>"#;

		assert_eq!(value.to_html(), expected);
	}

	#[test]
	pub fn test_rsx_element() {
		assert_eq!(
			parser(rsx_element)
				.parse(
					r#"<foo bar='baz'>
                            <foo bar='baz'>
                                <foo bar='baz'/>
                                hello<bar/>
                                world<baz/>
                            </foo>
                        </foo>"#
				)
				.unwrap(),
			(
				RsxElement::Normal(RsxNormalElement(
					RsxElementName::Name("foo".into()),
					RsxAttributes::from(vec![RsxAttribute::Named(
						RsxAttributeName::Name("bar".into()),
						RsxAttributeValue::Str("baz".into())
					),]),
					RsxChildren::from(vec![RsxChild::Element(
						RsxElement::Normal(RsxNormalElement(
							RsxElementName::Name("foo".into()),
							RsxAttributes::from(vec![RsxAttribute::Named(
								RsxAttributeName::Name("bar".into()),
								RsxAttributeValue::Str("baz".into())
							),]),
							RsxChildren::from(vec![
								RsxChild::Element(RsxElement::SelfClosing(
									RsxSelfClosingElement(
										RsxElementName::Name("foo".into()),
										RsxAttributes::from(vec![
											RsxAttribute::Named(
												RsxAttributeName::Name(
													"bar".into()
												),
												RsxAttributeValue::Str(
													"baz".into()
												)
											),
										])
									)
								)),
								RsxChild::Text(RsxText("hello".into())),
								RsxChild::Element(RsxElement::SelfClosing(
									RsxSelfClosingElement(
										RsxElementName::Name("bar".into()),
										RsxAttributes::from(vec![])
									)
								)),
								RsxChild::Text(RsxText("world".into())),
								RsxChild::Element(RsxElement::SelfClosing(
									RsxSelfClosingElement(
										RsxElementName::Name("baz".into()),
										RsxAttributes::from(vec![])
									)
								)),
							])
						))
					),])
				)),
				""
			)
		);
	}

	#[test]
	pub fn test_rsx_normal_element() {
		assert_eq!(parser(rsx_normal_element).parse("").is_err(), true);
		assert_eq!(parser(rsx_normal_element).parse(" ").is_err(), true);
		assert_eq!(parser(rsx_normal_element).parse("<foo>").is_err(), true);
		assert_eq!(parser(rsx_normal_element).parse("<foo/>").is_err(), true);
		assert_eq!(parser(rsx_normal_element).parse("< foo />").is_err(), true);
		assert_eq!(parser(rsx_normal_element).parse("<foo/ >").is_err(), true);
		assert_eq!(parser(rsx_normal_element).parse("</foo>").is_err(), true);
		assert_eq!(
			parser(rsx_normal_element).parse("<foo-bar/>").is_err(),
			true
		);
		assert_eq!(
			parser(rsx_normal_element).parse("<foo:bar/>").is_err(),
			true
		);
		assert_eq!(
			parser(rsx_normal_element).parse("<foo.bar/>").is_err(),
			true
		);

		assert_eq!(
			parser(rsx_normal_element).parse("<foo></foo>").unwrap(),
			("foo".into(), "")
		);
		assert_eq!(
			parser(rsx_normal_element).parse("< foo ></foo>").unwrap(),
			("foo".into(), "")
		);
		assert_eq!(
			parser(rsx_normal_element).parse("<foo></ foo >").unwrap(),
			("foo".into(), "")
		);
		assert_eq!(
			parser(rsx_normal_element).parse("< foo ></ foo >").unwrap(),
			("foo".into(), "")
		);

		assert_eq!(
			parser(rsx_normal_element)
				.parse("<foo-bar></foo-bar>")
				.unwrap(),
			("foo-bar".into(), "")
		);
		assert_eq!(
			parser(rsx_normal_element)
				.parse("<foo:bar></foo:bar>")
				.unwrap(),
			(("foo", "bar").into(), "")
		);
		assert_eq!(
			parser(rsx_normal_element)
				.parse("<foo.bar></foo.bar>")
				.unwrap(),
			(vec!["foo", "bar"][..].into(), "")
		);
	}

	#[test]
	pub fn test_rsx_normal_element_with_attributes() {
		assert_eq!(
			parser(rsx_normal_element).parse("<foo bar></foo>").unwrap(),
			(
				RsxNormalElement(
					RsxElementName::Name("foo".into()),
					RsxAttributes::from(vec![RsxAttribute::Named(
						RsxAttributeName::Name("bar".into()),
						RsxAttributeValue::Default
					),]),
					RsxChildren::from(vec![])
				),
				""
			)
		);

		assert_eq!(
			parser(rsx_normal_element)
				.parse("< foo bar ></ foo >")
				.unwrap(),
			(
				RsxNormalElement(
					RsxElementName::Name("foo".into()),
					RsxAttributes::from(vec![RsxAttribute::Named(
						RsxAttributeName::Name("bar".into()),
						RsxAttributeValue::Default
					),]),
					RsxChildren::from(vec![])
				),
				""
			)
		);

		assert_eq!(
			parser(rsx_normal_element)
				.parse("<foo bar baz></foo>")
				.unwrap(),
			(
				RsxNormalElement(
					RsxElementName::Name("foo".into()),
					RsxAttributes::from(vec![
						RsxAttribute::Named(
							RsxAttributeName::Name("bar".into()),
							RsxAttributeValue::Default
						),
						RsxAttribute::Named(
							RsxAttributeName::Name("baz".into()),
							RsxAttributeValue::Default
						),
					]),
					RsxChildren::from(vec![])
				),
				""
			)
		);

		assert_eq!(
			parser(rsx_normal_element)
				.parse("< foo bar baz ></ foo >")
				.unwrap(),
			(
				RsxNormalElement(
					RsxElementName::Name("foo".into()),
					RsxAttributes::from(vec![
						RsxAttribute::Named(
							RsxAttributeName::Name("bar".into()),
							RsxAttributeValue::Default
						),
						RsxAttribute::Named(
							RsxAttributeName::Name("baz".into()),
							RsxAttributeValue::Default
						),
					]),
					RsxChildren::from(vec![])
				),
				""
			)
		);

		assert_eq!(
			parser(rsx_normal_element).parse("<f b='z'></f>").unwrap(),
			(
				RsxNormalElement(
					RsxElementName::Name("f".into()),
					RsxAttributes::from(vec![RsxAttribute::Named(
						RsxAttributeName::Name("b".into()),
						RsxAttributeValue::Str("z".into())
					),]),
					RsxChildren::from(vec![])
				),
				""
			)
		);

		assert_eq!(
			parser(rsx_normal_element)
				.parse("< f b = 'z' ></ f >")
				.unwrap(),
			(
				RsxNormalElement(
					RsxElementName::Name("f".into()),
					RsxAttributes::from(vec![RsxAttribute::Named(
						RsxAttributeName::Name("b".into()),
						RsxAttributeValue::Str("z".into())
					),]),
					RsxChildren::from(vec![])
				),
				""
			)
		);
	}

	#[test]
	pub fn test_rsx_self_closing_element() {
		assert_eq!(parser(rsx_self_closing_element).parse("").is_err(), true);
		assert_eq!(parser(rsx_self_closing_element).parse(" ").is_err(), true);
		assert_eq!(
			parser(rsx_self_closing_element).parse("<foo>").is_err(),
			true
		);
		assert_eq!(
			parser(rsx_self_closing_element).parse("<foo/>").unwrap(),
			("foo".into(), "")
		);
		assert_eq!(
			parser(rsx_self_closing_element).parse("< foo />").unwrap(),
			("foo".into(), "")
		);
		assert_eq!(
			parser(rsx_self_closing_element).parse("<foo/ >").unwrap(),
			("foo".into(), "")
		);
		assert_eq!(
			parser(rsx_self_closing_element).parse("</foo>").is_err(),
			true
		);
		assert_eq!(
			parser(rsx_self_closing_element)
				.parse("<foo-bar/>")
				.unwrap(),
			("foo-bar".into(), "")
		);
		assert_eq!(
			parser(rsx_self_closing_element)
				.parse("<foo:bar/>")
				.unwrap(),
			(("foo", "bar").into(), "")
		);
		assert_eq!(
			parser(rsx_self_closing_element)
				.parse("<foo.bar/>")
				.unwrap(),
			(vec!["foo", "bar"][..].into(), "")
		);
	}

	#[test]
	pub fn test_rsx_self_closing_element_with_attributes() {
		assert_eq!(
			parser(rsx_self_closing_element)
				.parse("<foo bar/>")
				.unwrap(),
			(
				RsxSelfClosingElement(
					RsxElementName::Name("foo".into()),
					RsxAttributes::from(vec![RsxAttribute::Named(
						RsxAttributeName::Name("bar".into()),
						RsxAttributeValue::Default
					),])
				),
				""
			)
		);

		assert_eq!(
			parser(rsx_self_closing_element)
				.parse("< foo bar />")
				.unwrap(),
			(
				RsxSelfClosingElement(
					RsxElementName::Name("foo".into()),
					RsxAttributes::from(vec![RsxAttribute::Named(
						RsxAttributeName::Name("bar".into()),
						RsxAttributeValue::Default
					),])
				),
				""
			)
		);

		assert_eq!(
			parser(rsx_self_closing_element)
				.parse("<foo bar baz/>")
				.unwrap(),
			(
				RsxSelfClosingElement(
					RsxElementName::Name("foo".into()),
					RsxAttributes::from(vec![
						RsxAttribute::Named(
							RsxAttributeName::Name("bar".into()),
							RsxAttributeValue::Default
						),
						RsxAttribute::Named(
							RsxAttributeName::Name("baz".into()),
							RsxAttributeValue::Default
						),
					])
				),
				""
			)
		);

		assert_eq!(
			parser(rsx_self_closing_element)
				.parse("< foo bar baz />")
				.unwrap(),
			(
				RsxSelfClosingElement(
					RsxElementName::Name("foo".into()),
					RsxAttributes::from(vec![
						RsxAttribute::Named(
							RsxAttributeName::Name("bar".into()),
							RsxAttributeValue::Default
						),
						RsxAttribute::Named(
							RsxAttributeName::Name("baz".into()),
							RsxAttributeValue::Default
						),
					])
				),
				""
			)
		);

		assert_eq!(
			parser(rsx_self_closing_element)
				.parse("<f b='z'/>")
				.unwrap(),
			(
				RsxSelfClosingElement(
					RsxElementName::Name("f".into()),
					RsxAttributes::from(vec![RsxAttribute::Named(
						RsxAttributeName::Name("b".into()),
						RsxAttributeValue::Str("z".into())
					),])
				),
				""
			)
		);

		assert_eq!(
			parser(rsx_self_closing_element)
				.parse("< f b = 'z' />")
				.unwrap(),
			(
				RsxSelfClosingElement(
					RsxElementName::Name("f".into()),
					RsxAttributes::from(vec![RsxAttribute::Named(
						RsxAttributeName::Name("b".into()),
						RsxAttributeValue::Str("z".into())
					),])
				),
				""
			)
		);
	}

	#[test]
	pub fn test_rsx_opening_element() {
		assert_eq!(parser(rsx_opening_element).parse("").is_err(), true);
		assert_eq!(parser(rsx_opening_element).parse(" ").is_err(), true);
		assert_eq!(
			parser(rsx_opening_element).parse("<foo>").unwrap(),
			("foo".into(), "")
		);
		assert_eq!(
			parser(rsx_opening_element).parse("< foo >").unwrap(),
			("foo".into(), "")
		);
		assert_eq!(parser(rsx_opening_element).parse("<foo/>").is_err(), true);
		assert_eq!(parser(rsx_opening_element).parse("</foo>").is_err(), true);
		assert_eq!(
			parser(rsx_opening_element).parse("<foo-bar>").unwrap(),
			("foo-bar".into(), "")
		);
		assert_eq!(
			parser(rsx_opening_element).parse("<foo:bar>").unwrap(),
			(("foo", "bar").into(), "")
		);
		assert_eq!(
			parser(rsx_opening_element).parse("<foo.bar>").unwrap(),
			(vec!["foo", "bar"][..].into(), "")
		);
	}

	#[test]
	pub fn test_rsx_opening_element_with_attributes() {
		assert_eq!(
			parser(rsx_opening_element).parse("<foo bar>").unwrap(),
			(
				RsxOpeningElement(
					RsxElementName::Name("foo".into()),
					RsxAttributes::from(vec![RsxAttribute::Named(
						RsxAttributeName::Name("bar".into()),
						RsxAttributeValue::Default
					),])
				),
				""
			)
		);

		assert_eq!(
			parser(rsx_opening_element).parse("< foo bar >").unwrap(),
			(
				RsxOpeningElement(
					RsxElementName::Name("foo".into()),
					RsxAttributes::from(vec![RsxAttribute::Named(
						RsxAttributeName::Name("bar".into()),
						RsxAttributeValue::Default
					),])
				),
				""
			)
		);

		assert_eq!(
			parser(rsx_opening_element).parse("<foo bar baz>").unwrap(),
			(
				RsxOpeningElement(
					RsxElementName::Name("foo".into()),
					RsxAttributes::from(vec![
						RsxAttribute::Named(
							RsxAttributeName::Name("bar".into()),
							RsxAttributeValue::Default
						),
						RsxAttribute::Named(
							RsxAttributeName::Name("baz".into()),
							RsxAttributeValue::Default
						),
					])
				),
				""
			)
		);

		assert_eq!(
			parser(rsx_opening_element)
				.parse("< foo bar baz >")
				.unwrap(),
			(
				RsxOpeningElement(
					RsxElementName::Name("foo".into()),
					RsxAttributes::from(vec![
						RsxAttribute::Named(
							RsxAttributeName::Name("bar".into()),
							RsxAttributeValue::Default
						),
						RsxAttribute::Named(
							RsxAttributeName::Name("baz".into()),
							RsxAttributeValue::Default
						),
					])
				),
				""
			)
		);

		assert_eq!(
			parser(rsx_opening_element).parse("<f b='z'>").unwrap(),
			(
				RsxOpeningElement(
					RsxElementName::Name("f".into()),
					RsxAttributes::from(vec![RsxAttribute::Named(
						RsxAttributeName::Name("b".into()),
						RsxAttributeValue::Str("z".into())
					),])
				),
				""
			)
		);

		assert_eq!(
			parser(rsx_opening_element).parse("< f b = 'z' >").unwrap(),
			(
				RsxOpeningElement(
					RsxElementName::Name("f".into()),
					RsxAttributes::from(vec![RsxAttribute::Named(
						RsxAttributeName::Name("b".into()),
						RsxAttributeValue::Str("z".into())
					),])
				),
				""
			)
		);
	}

	#[test]
	pub fn test_rsx_closing_element() {
		assert_eq!(
			env_p(&"".into(), rsx_closing_element).parse("").is_err(),
			true
		);
		assert_eq!(
			env_p(&"".into(), rsx_closing_element).parse(" ").is_err(),
			true
		);
		assert_eq!(
			env_p(&"foo".into(), rsx_closing_element)
				.parse("<foo>")
				.is_err(),
			true
		);
		assert_eq!(
			env_p(&"foo".into(), rsx_closing_element)
				.parse("<foo/>")
				.is_err(),
			true
		);
		assert_eq!(
			env_p(&"foo".into(), rsx_closing_element)
				.parse("</foo>")
				.is_err(),
			false
		);
		assert_eq!(
			env_p(&"baz".into(), rsx_closing_element)
				.parse("</foo>")
				.err()
				.unwrap()
				.to_string()
				.contains("Unexpected `f`\nExpected `baz`\n"),
			true
		);
		assert_eq!(
			env_p(&"foo".into(), rsx_closing_element)
				.parse("</ foo >")
				.is_err(),
			false
		);
		assert_eq!(
			env_p(&"baz".into(), rsx_closing_element)
				.parse("</ foo >")
				.err()
				.unwrap()
				.to_string()
				.contains("Unexpected `f`\nExpected `baz`\n"),
			true
		);
		assert_eq!(
			env_p(&"foo".into(), rsx_closing_element)
				.parse("< /foo>")
				.is_err(),
			false
		);
		assert_eq!(
			env_p(&"baz".into(), rsx_closing_element)
				.parse("< /foo>")
				.err()
				.unwrap()
				.to_string()
				.contains("Unexpected `f`\nExpected `baz`\n"),
			true
		);
		assert_eq!(
			env_p(&"foo-bar".into(), rsx_closing_element)
				.parse("</foo-bar>")
				.is_err(),
			false
		);
		assert_eq!(
			env_p(&"foo-baz".into(), rsx_closing_element)
				.parse("</foo-bar>")
				.err()
				.unwrap()
				.to_string()
				.contains("Unexpected `r`\nExpected `foo-baz`\n"),
			true
		);
		assert_eq!(
			env_p(&"foo:bar".into(), rsx_closing_element)
				.parse("</foo:bar>")
				.is_err(),
			false
		);
		assert_eq!(
			env_p(&"foo:baz".into(), rsx_closing_element)
				.parse("</foo:bar>")
				.err()
				.unwrap()
				.to_string()
				.contains("Unexpected `r`\nExpected `foo:baz`\n"),
			true
		);
		assert_eq!(
			env_p(&"foo.bar".into(), rsx_closing_element)
				.parse("</foo.bar>")
				.is_err(),
			false
		);
		assert_eq!(
			env_p(&"foo.baz".into(), rsx_closing_element)
				.parse("</foo.bar>")
				.err()
				.unwrap()
				.to_string()
				.contains("Unexpected `r`\nExpected `foo.baz`\n"),
			true
		);
	}

	#[test]
	pub fn test_rsx_element_name() {
		assert_eq!(parser(rsx_element_name).parse("").is_err(), true);
		assert_eq!(parser(rsx_element_name).parse(" ").is_err(), true);
		assert_eq!(
			parser(rsx_element_name).parse("foo").unwrap(),
			("foo".into(), "")
		);
		assert_eq!(
			parser(rsx_element_name).parse("foo-bar").unwrap(),
			("foo-bar".into(), "")
		);
		assert_eq!(
			parser(rsx_element_name).parse("foo:bar").unwrap(),
			(("foo", "bar").into(), "")
		);
		assert_eq!(
			p(rsx_element_name).parse("foo.bar").unwrap(),
			(vec!["foo", "bar"][..].into(), "")
		);
	}

	#[test]
	pub fn test_rsx_identifier_simple() {
		assert_eq!(parser(rsx_identifier_simple).parse("").is_err(), true);
		assert_eq!(parser(rsx_identifier_simple).parse(" ").is_err(), true);
		assert_eq!(
			parser(rsx_identifier_simple).parse("foo").unwrap(),
			("foo".into(), "")
		);
		assert_eq!(
			parser(rsx_identifier_simple).parse("foo_bar").unwrap(),
			("foo_bar".into(), "")
		);
		assert_eq!(
			parser(rsx_identifier_simple).parse("foo$bar").unwrap(),
			("foo$bar".into(), "")
		);
		assert_eq!(parser(rsx_identifier_simple).parse("1foo").is_err(), true);
		assert_eq!(
			parser(rsx_identifier_simple).parse("foo1").unwrap(),
			("foo1".into(), "")
		);
	}

	#[test]
	pub fn test_rsx_identifier() {
		assert_eq!(parser(rsx_identifier).parse("").is_err(), true);
		assert_eq!(parser(rsx_identifier).parse(" ").is_err(), true);
		assert_eq!(
			parser(rsx_identifier).parse("foo").unwrap(),
			("foo".into(), "")
		);
		assert_eq!(
			parser(rsx_identifier).parse("foo-bar").unwrap(),
			("foo-bar".into(), "")
		);
		assert_eq!(
			parser(rsx_identifier).parse("$foo-$bar").unwrap(),
			("$foo-$bar".into(), "")
		);
		assert_eq!(parser(rsx_identifier).parse("1foo-bar").is_err(), true);
		assert_eq!(
			parser(rsx_identifier).parse("foo1-bar").unwrap(),
			("foo1-bar".into(), "")
		);
		assert_eq!(parser(rsx_identifier).parse("foo-1bar").is_err(), true);
		assert_eq!(
			parser(rsx_identifier).parse("foo-bar1").unwrap(),
			("foo-bar1".into(), "")
		);
	}

	#[test]
	pub fn test_rsx_namespaced_name() {
		assert_eq!(parser(rsx_namespaced_name).parse("").is_err(), true);
		assert_eq!(parser(rsx_namespaced_name).parse(" ").is_err(), true);
		assert_eq!(parser(rsx_namespaced_name).parse("foo").is_err(), true);
		assert_eq!(
			p(rsx_namespaced_name).parse("foo:bar").unwrap(),
			(("foo".into(), "bar".into()), "")
		);
		assert_eq!(
			p(rsx_namespaced_name).parse("$foo:$bar").unwrap(),
			(("$foo".into(), "$bar".into()), "")
		);
		assert_eq!(
			p(rsx_namespaced_name).parse("foo:bar:baz").unwrap(),
			(("foo".into(), "bar".into()), ":baz")
		);
		assert_eq!(
			p(rsx_namespaced_name).parse("$foo:$bar:$baz").unwrap(),
			(("$foo".into(), "$bar".into()), ":$baz")
		);
		assert_eq!(
			parser(rsx_namespaced_name).parse("1foo:bar").is_err(),
			true
		);
		assert_eq!(
			p(rsx_namespaced_name).parse("foo1:bar").unwrap(),
			(("foo1".into(), "bar".into()), "")
		);
		assert_eq!(
			parser(rsx_namespaced_name).parse("foo:1bar").is_err(),
			true
		);
		assert_eq!(
			p(rsx_namespaced_name).parse("foo:bar1").unwrap(),
			(("foo".into(), "bar1".into()), "")
		);
	}

	#[test]
	pub fn test_rsx_member_expression() {
		assert_eq!(parser(rsx_member_expression).parse("").is_err(), true);
		assert_eq!(parser(rsx_member_expression).parse(" ").is_err(), true);
		assert_eq!(parser(rsx_member_expression).parse("foo").is_err(), true);
		assert_eq!(
			p(rsx_member_expression)
				.parse("foo.bar")
				.unwrap()
				.0
				.into_vec(),
			vec!["foo".into(), "bar".into()]
		);
		assert_eq!(
			p(rsx_member_expression)
				.parse("$foo.$bar")
				.unwrap()
				.0
				.into_vec(),
			vec!["$foo".into(), "$bar".into()]
		);
		assert_eq!(
			p(rsx_member_expression)
				.parse("foo.bar.baz")
				.unwrap()
				.0
				.into_vec(),
			vec!["foo".into(), "bar".into(), "baz".into()]
		);
		assert_eq!(
			p(rsx_member_expression)
				.parse("$foo.$bar.$baz")
				.unwrap()
				.0
				.into_vec(),
			vec!["$foo".into(), "$bar".into(), "$baz".into()]
		);
		assert_eq!(
			parser(rsx_member_expression).parse("1foo.bar").is_err(),
			true
		);
		assert_eq!(
			p(rsx_member_expression)
				.parse("foo1.bar")
				.unwrap()
				.0
				.into_vec(),
			vec!["foo1".into(), "bar".into()]
		);
		assert_eq!(
			parser(rsx_member_expression).parse("foo.1bar").is_err(),
			true
		);
		assert_eq!(
			p(rsx_member_expression)
				.parse("foo.bar1")
				.unwrap()
				.0
				.into_vec(),
			vec!["foo".into(), "bar1".into()]
		);
	}

	use sweet::prelude::*;

	#[test]
	pub fn test_rsx_fragment_empty() {
		let result = p(rsx_fragment).parse("<></>");
		expect(result.is_ok()).to_be_true();
		let (fragment, _) = result.unwrap();
		expect(fragment.0.0.len()).to_be(0);
	}

	#[test]
	pub fn test_rsx_fragment_with_content() {
		let result = p(rsx_fragment).parse("<>Hello World</>");
		expect(result.is_ok()).to_be_true();
		let (fragment, _) = result.unwrap();
		expect(fragment.0.0.len()).to_be(1);
	}

	#[test]
	pub fn test_rsx_fragment_with_elements() {
		let result =
			p(rsx_fragment).parse("<><div>content</div><span>more</span></>");
		expect(result.is_ok()).to_be_true();
		let (fragment, _) = result.unwrap();
		expect(fragment.0.0.len()).to_be(2);
	}

	#[test]
	pub fn test_rsx_element_includes_fragment() {
		let result = p(rsx_element).parse("<></>");
		expect(result.is_ok()).to_be_true();
		let (element, _) = result.unwrap();
		match element {
			RsxElement::Fragment(_) => {}
			_ => panic!("Expected fragment element"),
		}
	}
}
