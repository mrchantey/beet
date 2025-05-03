use crate::prelude::*;
use combine::ParseResult;
use combine::Parser;
use combine::Stream;
use combine::char::string;
use combine::choice;
use combine::combinator::between;
use combine::combinator::many;
use combine::combinator::many1;
use combine::combinator::none_of;
use combine::combinator::optional;
use combine::combinator::parser;
use combine::combinator::token;
use combine::combinator::r#try;
use combine::combinator::value;

pub fn rsx_code_block_begin<I>(input: I) -> ParseResult<(), I>
where
	I: Stream<Item = char>,
{
	token('{').with(value(())).parse_stream(input)
}

pub fn rsx_code_block_end<I>(input: I) -> ParseResult<(), I>
where
	I: Stream<Item = char>,
{
	token('}').with(value(())).parse_stream(input)
}

pub fn rsx_code_block<I>(input: I) -> ParseResult<RsxParsedExpression, I>
where
	I: Stream<Item = char>,
{
	between(
		parser(rsx_code_block_begin),
		parser(rsx_code_block_end),
		many(parser(rsx_code_block_fragment)),
	)
	.parse_stream(input)
}



pub fn rsx_spread_code_block<I>(input: I) -> ParseResult<RsxParsedExpression, I>
where
	I: Stream<Item = char>,
{
	between(
		parser(rsx_code_block_begin),
		parser(rsx_code_block_end),
		(optional(parser(rs_whitespace)), string("..."))
			.with(many1(parser(rsx_code_block_fragment))),
	)
	.parse_stream(input)
}

pub fn rsx_code_block_fragment<I>(
	input: I,
) -> ParseResult<RsxRawCodeFragment, I>
where
	I: Stream<Item = char>,
{
	choice!(
		r#try(parser(rsx_code_block).map(RsxRawCodeFragment::ParsedExpression)),
		r#try(parser(rsx_element).map(RsxRawCodeFragment::Element)),
		r#try(parser(rs_comment).map(|_| RsxRawCodeFragment::Empty)),
		r#try(
			parser(rs_char)
				.map(|v| RsxRawCodeFragment::Tokens(format!("'{}'", v.0)))
		),
		r#try(
			parser(rs_string)
				.map(|v| RsxRawCodeFragment::Tokens(format!("\"{}\"", v.0)))
		),
		none_of("}".chars()).map(RsxRawCodeFragment::Token)
	)
	.parse_stream(input)
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use combine::Parser;
	use combine::combinator::parser;
	use sweet::prelude::*;

	#[test]
	pub fn test_rsx_code_block_to_html() {
		let value = parser(rsx_code_block)
			.parse(
				r#"{
                    if foo {
                        <bar {...props}>
                            hello
                            {
                                name(
                                    <world/>
                                )
                            }
                        </bar>
                    } else {
                        <baz {...props}>
                            goodbye
                            {
                                name(
                                    <blue-sky/>
                                )
                            }
                        </baz>
                    }
                    <what { ...
                        if happy_and_you_know_it {
                            <clap {...props}/>
                        } else {
                            <dont.clap {...props}/>
                        }
                    }/>
                }"#,
			)
			.unwrap()
			.0;


		let expected = r#"{
                    if foo {
                        <bar {props}>hello{
                                name(
                                    <world/>
                                )
                            }</bar>
                    } else {
                        <baz {props}>goodbye{
                                name(
                                    <blue-sky/>
                                )
                            }</baz>
                    }
                    <what {
                        if happy_and_you_know_it {
                            <clap {props}/>
                        } else {
                            <dont.clap {props}/>
                        }
                    }/>
                }"#;
		assert_eq!(value.to_html(), expected);
	}
	#[test]
	pub fn test_rsx_code_block() {
		assert_eq!(parser(rsx_code_block).parse("").is_err(), true);
		assert_eq!(parser(rsx_code_block).parse(" ").is_err(), true);
		assert_eq!(parser(rsx_code_block).parse("foo").is_err(), true);
		assert_eq!(
			parser(rsx_code_block).parse("{}").unwrap(),
			(RsxParsedExpression(vec![]), "")
		);
		assert_eq!(
			parser(rsx_code_block).parse("{{}}").unwrap(),
			(RsxParsedExpression(vec!["{}".into()]), "")
		);
		assert_eq!(
			parser(rsx_code_block).parse("{foo}").unwrap(),
			(RsxParsedExpression(vec!["foo".into()]), "")
		);
		assert_eq!(
			parser(rsx_code_block).parse("{ foo }").unwrap(),
			(RsxParsedExpression(vec![" foo ".into()]), "")
		);
		assert_eq!(
			parser(rsx_code_block).parse("{{foo}}").unwrap(),
			(RsxParsedExpression(vec!["{foo}".into()]), "")
		);
		assert_eq!(
			parser(rsx_code_block).parse("{ { foo } }").unwrap(),
			(RsxParsedExpression(vec![" { foo } ".into()]), "")
		);
		assert_eq!(
			parser(rsx_code_block).parse("{ foo bar baz }").unwrap(),
			(RsxParsedExpression(vec![" foo bar baz ".into()]), "")
		);
		assert_eq!(
			parser(rsx_code_block)
				.parse("{ if foo { bar } else { baz } }")
				.unwrap(),
			(
				RsxParsedExpression(vec![
					" if foo { bar } else { baz } ".into()
				]),
				""
			)
		);
		assert_eq!(
			parser(rsx_code_block)
				.parse("{ if foo { \"bar\" } else { \"baz\" } }")
				.unwrap(),
			(
				RsxParsedExpression(vec![
					" if foo { \"bar\" } else { \"baz\" } ".into()
				]),
				""
			)
		);
		assert_eq!(
			parser(rsx_code_block)
				.parse("{ if foo { \"{bar}\" } else { \"{baz}\" } }")
				.unwrap(),
			(
				RsxParsedExpression(vec![
					" if foo { \"{bar}\" } else { \"{baz}\" } ".into()
				]),
				""
			)
		);
		assert_eq!(
			parser(rsx_code_block)
				.parse("{ if foo { /* {bar} */ } else { /* {baz} */ } }")
				.unwrap(),
			(
				RsxParsedExpression(vec![" if foo {  } else {  } ".into()]),
				""
			)
		);
		assert_eq!(
			parser(rsx_code_block)
				.parse("{ if foo { <bar/> } else { <baz/> } }")
				.unwrap(),
			(
				RsxParsedExpression(vec![
					" if foo { ".into(),
					RsxElement::SelfClosing(RsxSelfClosingElement(
						RsxElementName::Name("bar".into()),
						RsxAttributes::from(vec![])
					))
					.into(),
					" } else { ".into(),
					RsxElement::SelfClosing(RsxSelfClosingElement(
						RsxElementName::Name("baz".into()),
						RsxAttributes::from(vec![])
					))
					.into(),
					" } ".into(),
				]),
				""
			)
		);
		assert_eq!(
			parser(rsx_code_block)
				.parse(
					"{ if foo { <bar>{ hello }</bar> } else { <baz>{ world }</baz> } }"
				)
				.unwrap(),
			(
				RsxParsedExpression(vec![
					" if foo { ".into(),
					RsxElement::Normal(RsxNormalElement(
						RsxElementName::Name("bar".into()),
						RsxAttributes::from(vec![]),
						RsxChildren::from(vec![RsxChild::CodeBlock(
							RsxParsedExpression(vec![" hello ".into()])
						),])
					))
					.into(),
					" } else { ".into(),
					RsxElement::Normal(RsxNormalElement(
						RsxElementName::Name("baz".into()),
						RsxAttributes::from(vec![]),
						RsxChildren::from(vec![RsxChild::CodeBlock(
							RsxParsedExpression(vec![" world ".into()])
						),])
					))
					.into(),
					" } ".into(),
				]),
				""
			)
		);
		assert_eq!(
			parser(rsx_code_block)
				.parse(
					"{ if foo { <bar>{ <hello/> }</bar> } else { <baz>{ <world/> }</baz> } }"
				)
				.unwrap(),
			(
				RsxParsedExpression(vec![
					" if foo { ".into(),
					RsxElement::Normal(RsxNormalElement(
						RsxElementName::Name("bar".into()),
						RsxAttributes::from(vec![]),
						RsxChildren::from(vec![RsxChild::CodeBlock(
							RsxParsedExpression(vec![
								" ".into(),
								RsxElement::SelfClosing(RsxSelfClosingElement(
									RsxElementName::Name("hello".into()),
									RsxAttributes::from(vec![])
								))
								.into(),
								" ".into(),
							])
						),])
					))
					.into(),
					" } else { ".into(),
					RsxElement::Normal(RsxNormalElement(
						RsxElementName::Name("baz".into()),
						RsxAttributes::from(vec![]),
						RsxChildren::from(vec![RsxChild::CodeBlock(
							RsxParsedExpression(vec![
								" ".into(),
								RsxElement::SelfClosing(RsxSelfClosingElement(
									RsxElementName::Name("world".into()),
									RsxAttributes::from(vec![])
								))
								.into(),
								" ".into(),
							])
						),])
					))
					.into(),
					" } ".into(),
				]),
				""
			)
		);
		assert_eq!(
					parser(rsx_code_block)
							.parse(
									"{ if foo { <bar>{ lorem(<hello/>) }</bar> } else { <baz>{ ipsum(<world/>) }</baz> } }"
							)
							.unwrap(),
					(
							RsxParsedExpression(vec![
									" if foo { ".into(),
									RsxElement::Normal(RsxNormalElement(
											RsxElementName::Name("bar".into()),
											RsxAttributes::from(vec![]),
											RsxChildren::from(vec![
													RsxChild::CodeBlock(RsxParsedExpression(vec![
															" lorem(".into(),
															RsxElement::SelfClosing(RsxSelfClosingElement(
																	RsxElementName::Name("hello".into()),
																	RsxAttributes::from(vec![])
															)).into(),
															") ".into(),
													])),
											])
									)).into(),
									" } else { ".into(),
									RsxElement::Normal(RsxNormalElement(
											RsxElementName::Name("baz".into()),
											RsxAttributes::from(vec![]),
											RsxChildren::from(vec![
													RsxChild::CodeBlock(RsxParsedExpression(vec![
															" ipsum(".into(),
															RsxElement::SelfClosing(RsxSelfClosingElement(
																	RsxElementName::Name("world".into()),
																	RsxAttributes::from(vec![])
															)).into(),
															") ".into(),
													])),
											])
									)).into(),
									" } ".into(),
							]),
							""
					)
			);
	}

	#[test]
	pub fn test_rsx_spread_code_block() {
		expect(parser(rsx_spread_code_block).parse("").is_err()).to_be(true);
		expect(parser(rsx_spread_code_block).parse(" ").is_err()).to_be(true);
		expect(parser(rsx_spread_code_block).parse("foo").is_err()).to_be(true);
		expect(parser(rsx_spread_code_block).parse("{...}").is_err())
			.to_be(true);
		expect(parser(rsx_spread_code_block).parse("{...foo}").unwrap())
			.to_be((RsxParsedExpression(vec!["foo".into()]), ""));
		expect(parser(rsx_spread_code_block).parse("{ ... foo }").unwrap())
			.to_be((RsxParsedExpression(vec![" foo ".into()]), ""));
		expect(parser(rsx_spread_code_block).parse("{...{foo}}").unwrap())
			.to_be((RsxParsedExpression(vec!["{foo}".into()]), ""));
		expect(
			parser(rsx_spread_code_block)
				.parse("{ ... { foo } }")
				.unwrap(),
		)
		.to_be((RsxParsedExpression(vec![" { foo } ".into()]), ""));
	}
}
