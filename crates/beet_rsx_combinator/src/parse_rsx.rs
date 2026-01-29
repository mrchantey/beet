use crate::prelude::*;
use combine::ParseResult;
use combine::Parser;
use combine::Stream;
use combine::combinator::optional;
use combine::combinator::parser;

pub fn rsx_element_ignoring_ws<I>(input: I) -> ParseResult<RsxElement, I>
where
	I: Stream<Item = char>,
{
	optional(parser(js_whitespace))
		.with(parser(rsx_element).skip(parser(js_whitespace)))
		.parse_stream(input)
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use combine::Parser;
	use combine::combinator::parser;

	#[test]
	pub fn test_rsx_simple_expression() {
		assert_eq!(
			parser(rsx_element_ignoring_ws)
				.parse(r#"<div>Hello world!</div>"#)
				.unwrap(),
			(
				RsxElement::Normal(RsxNormalElement(
					RsxElementName::Name("div".into()),
					RsxAttributes::from(vec![]),
					RsxChildren::from(vec![RsxChild::Text(RsxText(
						"Hello world!".into()
					))])
				)),
				""
			)
		);
	}

	#[test]
	pub fn test_rsx_complex() {
		parser(rsx_element_ignoring_ws)
			.parse(
				r#"
<root
	first
	second={true}
	third={false}
	fourth={1}
	fifth='2'
	sixth="3"
	seventh={'4'}
	eighth={"5"}
	ninth={6 + 7 - 8 * 9 / 10}
	tenth={|e: Event| { println!("{:?}", e); }}
>
	<div>hello</div>
	<span>world</span>
	{
		if foo {
				<first>lorem { 1 + 2 } ipsum</first>
		} else {
				<second>dolor { 3 + 4 } sit</second>
		}
	}
	<ul>
		<li>First</li>
		<li>Second</li>
		<li>Third</li>
	</ul>
	<void/>
</root>
								"#,
			)
			.unwrap()
			.xpect_eq((
				RsxElement::Normal(RsxNormalElement(
					RsxElementName::Name("root".into()),
					RsxAttributes::from(vec![
						RsxAttribute::Named(
							RsxAttributeName::Name("first".into()),
							RsxAttributeValue::Default,
						),
						RsxAttribute::Named(
							RsxAttributeName::Name("second".into()),
							RsxAttributeValue::Boolean(true.into()),
						),
						RsxAttribute::Named(
							RsxAttributeName::Name("third".into()),
							RsxAttributeValue::Boolean(false.into()),
						),
						RsxAttribute::Named(
							RsxAttributeName::Name("fourth".into()),
							RsxAttributeValue::Number(1f64.into()),
						),
						RsxAttribute::Named(
							RsxAttributeName::Name("fifth".into()),
							RsxAttributeValue::Str(("2", '"').into()),
						),
						RsxAttribute::Named(
							RsxAttributeName::Name("sixth".into()),
							RsxAttributeValue::Str(("3", '\'').into()),
						),
						RsxAttribute::Named(
							RsxAttributeName::Name("seventh".into()),
							RsxAttributeValue::Str(("4", '"').into()),
						),
						RsxAttribute::Named(
							RsxAttributeName::Name("eighth".into()),
							RsxAttributeValue::Str(("5", '\'').into()),
						),
						RsxAttribute::Named(
							RsxAttributeName::Name("ninth".into()),
							RsxAttributeValue::CodeBlock(RsxParsedExpression(
								vec!["6 + 7 - 8 * 9 / 10".into()],
							)),
						),
						RsxAttribute::Named(
							RsxAttributeName::Name("tenth".into()),
							RsxAttributeValue::CodeBlock(RsxParsedExpression(
								vec![
									"|e: Event| { println!(\"{:?}\", e); }"
										.into(),
								],
							)),
						),
					]),
					RsxChildren::from(vec![
						RsxChild::Element(RsxElement::Normal(
							RsxNormalElement(
								RsxElementName::Name("div".into()),
								RsxAttributes::from(vec![]),
								RsxChildren::from(vec![RsxChild::Text(
									RsxText("hello".into()),
								)]),
							),
						)),
						RsxChild::Element(RsxElement::Normal(
							RsxNormalElement(
								RsxElementName::Name("span".into()),
								RsxAttributes::from(vec![]),
								RsxChildren::from(vec![RsxChild::Text(
									RsxText("world".into()),
								)]),
							),
						)),
						RsxChild::CodeBlock(RsxParsedExpression(vec![
							"\n\t\tif foo {\n\t\t\t\t".into(),
							RsxElement::Normal(RsxNormalElement(
								RsxElementName::Name("first".into()),
								RsxAttributes::from(vec![]),
								RsxChildren::from(vec![
									RsxChild::Text(RsxText("lorem".into())),
									RsxChild::CodeBlock(RsxParsedExpression(
										vec![" 1 + 2 ".into()],
									)),
									RsxChild::Text(RsxText("ipsum".into())),
								]),
							))
							.into(),
							"\n\t\t} else {\n\t\t\t\t".into(),
							RsxElement::Normal(RsxNormalElement(
								RsxElementName::Name("second".into()),
								RsxAttributes::from(vec![]),
								RsxChildren::from(vec![
									RsxChild::Text(RsxText("dolor".into())),
									RsxChild::CodeBlock(RsxParsedExpression(
										vec![" 3 + 4 ".into()],
									)),
									RsxChild::Text(RsxText("sit".into())),
								]),
							))
							.into(),
							"\n\t\t}\n\t".into(),
						])),
						RsxChild::Element(RsxElement::Normal(
							RsxNormalElement(
								RsxElementName::Name("ul".into()),
								RsxAttributes::from(vec![]),
								RsxChildren::from(vec![
									RsxChild::Element(RsxElement::Normal(
										RsxNormalElement(
											RsxElementName::Name("li".into()),
											RsxAttributes::from(vec![]),
											RsxChildren::from(vec![
												RsxChild::Text(RsxText(
													"First".into(),
												)),
											]),
										),
									)),
									RsxChild::Element(RsxElement::Normal(
										RsxNormalElement(
											RsxElementName::Name("li".into()),
											RsxAttributes::from(vec![]),
											RsxChildren::from(vec![
												RsxChild::Text(RsxText(
													"Second".into(),
												)),
											]),
										),
									)),
									RsxChild::Element(RsxElement::Normal(
										RsxNormalElement(
											RsxElementName::Name("li".into()),
											RsxAttributes::from(vec![]),
											RsxChildren::from(vec![
												RsxChild::Text(RsxText(
													"Third".into(),
												)),
											]),
										),
									)),
								]),
							),
						)),
						RsxChild::Element(RsxElement::SelfClosing(
							RsxSelfClosingElement(
								RsxElementName::Name("void".into()),
								RsxAttributes::from(vec![]),
							),
						)),
					]),
				)),
				"",
			));
	}

	#[test]
	pub fn test_rsx_example() {
		parser(rsx_element_ignoring_ws)
			.parse(
				r#"
<Dropdown show={props.visible}>
	A dropdown list
	<Menu
		icon={props.menu.icon}
		onHide={|e| println!("{:?}", e)}
		onShow={|e| println!("{:?}", e)}
	>
		<MenuItem>Do Something</MenuItem>
		{
			if should_do_something_fun() {
				<MenuItem>Do Something Fun!</MenuItem>
			} else {
				<MenuItem>Do Something Else</MenuItem>
			}
		}
	</Menu>
</Dropdown>
								"#,
			)
			.unwrap()
			.xpect_eq((
				RsxElement::Normal(RsxNormalElement(
					RsxElementName::Name("Dropdown".into()),
					RsxAttributes::from(vec![RsxAttribute::Named(
						RsxAttributeName::Name("show".into()),
						RsxAttributeValue::CodeBlock(RsxParsedExpression(
							vec!["props.visible".into()],
						)),
					)]),
					RsxChildren::from(vec![
						RsxChild::Text(RsxText("A dropdown list".into())),
						RsxChild::Element(RsxElement::Normal(
							RsxNormalElement(
								RsxElementName::Name("Menu".into()),
								RsxAttributes::from(vec![
									RsxAttribute::Named(
										RsxAttributeName::Name("icon".into()),
										RsxAttributeValue::CodeBlock(
											RsxParsedExpression(vec![
												"props.menu.icon".into(),
											]),
										),
									),
									RsxAttribute::Named(
										RsxAttributeName::Name("onHide".into()),
										RsxAttributeValue::CodeBlock(
											RsxParsedExpression(vec![
												"|e| println!(\"{:?}\", e)"
													.into(),
											]),
										),
									),
									RsxAttribute::Named(
										RsxAttributeName::Name("onShow".into()),
										RsxAttributeValue::CodeBlock(
											RsxParsedExpression(vec![
												"|e| println!(\"{:?}\", e)"
													.into(),
											]),
										),
									),
								]),
								RsxChildren::from(vec![
									RsxChild::Element(RsxElement::Normal(
										RsxNormalElement(
											RsxElementName::Name(
												"MenuItem".into(),
											),
											RsxAttributes::from(vec![]),
											RsxChildren::from(vec![
												RsxChild::Text(RsxText(
													"Do Something".into(),
												)),
											]),
										),
									)),
									RsxChild::CodeBlock(RsxParsedExpression(
										vec![
								"\n\t\t\tif should_do_something_fun() {\n\t\t\t\t".into(),
								RsxElement::Normal(RsxNormalElement(
									RsxElementName::Name("MenuItem".into()),
									RsxAttributes::from(vec![]),
									RsxChildren::from(vec![RsxChild::Text(
										RsxText("Do Something Fun!".into()),
									)]),
								))
								.into(),
								"\n\t\t\t} else {\n\t\t\t\t".into(),
								RsxElement::Normal(RsxNormalElement(
									RsxElementName::Name("MenuItem".into()),
									RsxAttributes::from(vec![]),
									RsxChildren::from(vec![RsxChild::Text(
										RsxText("Do Something Else".into()),
									)]),
								))
								.into(),
								"\n\t\t\t}\n\t\t".into(),
							],
									)),
								]),
							),
						)),
					]),
				)),
				"",
			));
	}
}
