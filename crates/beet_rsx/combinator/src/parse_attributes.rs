use crate::prelude::*;
use combine::ParseResult;
use combine::Parser;
use combine::Stream;
use combine::choice;
use combine::combinator::between;
use combine::combinator::many1;
use combine::combinator::parser;
use combine::combinator::token;
use combine::combinator::r#try;

pub fn rsx_attributes<I>(input: I) -> ParseResult<RsxAttributes, I>
where
	I: Stream<Item = char>,
{
	many1(parser(rsx_attribute).skip(parser(js_whitespace))).parse_stream(input)
}

pub fn rsx_attribute<I>(input: I) -> ParseResult<RsxAttribute, I>
where
	I: Stream<Item = char>,
{
	choice!(
		r#try(parser(rsx_spread_attribute)),
		r#try(parser(rsx_custom_attribute)),
		parser(rsx_default_attribute)
	)
	.parse_stream(input)
}

pub fn rsx_spread_attribute<I>(input: I) -> ParseResult<RsxAttribute, I>
where
	I: Stream<Item = char>,
{
	parser(rsx_spread_code_block)
		.map(RsxAttribute::Spread)
		.parse_stream(input)
}

pub fn rsx_custom_attribute<I>(input: I) -> ParseResult<RsxAttribute, I>
where
	I: Stream<Item = char>,
{
	(
		parser(rsx_attribute_complex_name).skip(parser(js_whitespace)),
		token('=').skip(parser(js_whitespace)),
		parser(rsx_attribute_value),
	)
		.map(|(n, _, v)| RsxAttribute::Named(n, v))
		.parse_stream(input)
}

pub fn rsx_default_attribute<I>(input: I) -> ParseResult<RsxAttribute, I>
where
	I: Stream<Item = char>,
{
	parser(rsx_attribute_complex_name)
		.map(|n| RsxAttribute::Named(n, RsxAttributeValue::Default))
		.parse_stream(input)
}

pub fn rsx_attribute_complex_name<I>(
	input: I,
) -> ParseResult<RsxAttributeName, I>
where
	I: Stream<Item = char>,
{
	// namespaced name is deprecated, its just not rusty
	choice!(
		r#try(parser(rsx_namespaced_name).map(|(ns, n)| {
			RsxAttributeName::Name(RsxIdentifier(format!("{}:{}", ns.0, n.0)))
		})),
		parser(rsx_identifier).map(RsxAttributeName::Name)
	)
	.parse_stream(input)
}

pub fn rsx_attribute_value<I>(input: I) -> ParseResult<RsxAttributeValue, I>
where
	I: Stream<Item = char>,
{
	choice!(
		r#try(
			parser(rsx_bracketed_attribute_bool)
				.map(RsxAttributeValue::Boolean)
		),
		r#try(
			parser(rsx_bracketed_attribute_number)
				.map(RsxAttributeValue::Number)
		),
		r#try(
			parser(rsx_bracketed_string_characters).map(RsxAttributeValue::Str)
		),
		r#try(parser(rsx_code_block).map(RsxAttributeValue::CodeBlock)),
		parser(rsx_element).map(RsxAttributeValue::Element)
	)
	.parse_stream(input)
}

pub fn rsx_bracketed_attribute_bool<I>(
	input: I,
) -> ParseResult<RsxAttributeBoolean, I>
where
	I: Stream<Item = char>,
{
	choice!(
		parser(rsx_attribute_bool),
		between(
			token('{').skip(parser(js_whitespace)),
			token('}'),
			parser(rsx_attribute_bool).skip(parser(js_whitespace))
		)
	)
	.parse_stream(input)
}

pub fn rsx_attribute_bool<I>(input: I) -> ParseResult<RsxAttributeBoolean, I>
where
	I: Stream<Item = char>,
{
	parser(js_boolean)
		.map(RsxAttributeBoolean::from)
		.parse_stream(input)
}

pub fn rsx_bracketed_attribute_number<I>(
	input: I,
) -> ParseResult<RsxAttributeNumber, I>
where
	I: Stream<Item = char>,
{
	choice!(
		parser(rsx_attribute_number),
		between(
			token('{').skip(parser(js_whitespace)),
			token('}'),
			parser(rsx_attribute_number).skip(parser(js_whitespace))
		)
	)
	.parse_stream(input)
}

pub fn rsx_attribute_number<I>(input: I) -> ParseResult<RsxAttributeNumber, I>
where
	I: Stream<Item = char>,
{
	parser(js_number)
		.map(RsxAttributeNumber::from)
		.parse_stream(input)
}

pub fn rsx_bracketed_string_characters<I>(
	input: I,
) -> ParseResult<RsxAttributeString, I>
where
	I: Stream<Item = char>,
{
	choice!(
		parser(rsx_string_characters),
		between(
			token('{').skip(parser(js_whitespace)),
			token('}'),
			parser(rsx_string_characters)
		)
	)
	.parse_stream(input)
}

pub fn rsx_string_characters<I>(input: I) -> ParseResult<RsxAttributeString, I>
where
	I: Stream<Item = char>,
{
	choice!(
		r#try(
			parser(js_double_string_characters)
				.map(RsxAttributeString::DoubleQuoted)
		),
		parser(js_single_string_characters)
			.map(RsxAttributeString::SingleQuoted)
	)
	.skip(parser(js_whitespace))
	.parse_stream(input)
}

#[cfg(test)]
mod tests {
	use crate::to_html::ToHtml;

	use super::*;

	#[test]
	pub fn test_rsx_attributes_to_html() {
		// Basic attribute
		let value = parser(rsx_attributes).parse("attribute").unwrap().0;
		assert_eq!(value.to_html(), "attribute");

		// Single quotes
		let value = parser(rsx_attributes).parse("attribute='c'").unwrap().0;
		assert_eq!(value.to_html(), "attribute='c'");

		// Double quotes
		let value = parser(rsx_attributes).parse("attribute=\"s\"").unwrap().0;
		assert_eq!(value.to_html(), "attribute=\"s\"");

		// String values
		let value = parser(rsx_attributes).parse("attribute='bar'").unwrap().0;
		assert_eq!(value.to_html(), "attribute='bar'");

		let value =
			parser(rsx_attributes).parse("attribute=\"bar\"").unwrap().0;
		assert_eq!(value.to_html(), "attribute=\"bar\"");

		// Quotes in strings
		let value = parser(rsx_attributes)
			.parse("attribute='bar\"baz'")
			.unwrap()
			.0;
		assert_eq!(value.to_html(), "attribute='bar\"baz'");

		let value = parser(rsx_attributes)
			.parse("attribute='bar\\\"baz'")
			.unwrap()
			.0;
		assert_eq!(value.to_html(), "attribute='bar\"baz'");

		let value = parser(rsx_attributes)
			.parse("attribute='bar\\'baz'")
			.unwrap()
			.0;
		assert_eq!(value.to_html(), "attribute='bar'baz'");

		let value = parser(rsx_attributes)
			.parse("attribute='bar\\nbaz'")
			.unwrap()
			.0;
		assert_eq!(value.to_html(), "attribute='bar\nbaz'");

		let value = parser(rsx_attributes)
			.parse("attribute=\"bar'baz\"")
			.unwrap()
			.0;
		assert_eq!(value.to_html(), "attribute=\"bar'baz\"");

		let value = parser(rsx_attributes)
			.parse("attribute=\"bar\\'baz\"")
			.unwrap()
			.0;
		assert_eq!(value.to_html(), "attribute=\"bar'baz\"");

		let value = parser(rsx_attributes)
			.parse("attribute=\"bar\\\"baz\"")
			.unwrap()
			.0;
		assert_eq!(value.to_html(), "attribute=\"bar\"baz\"");

		let value = parser(rsx_attributes)
			.parse("attribute=\"bar\\nbaz\"")
			.unwrap()
			.0;
		assert_eq!(value.to_html(), "attribute=\"bar\nbaz\"");

		// JSX character expressions
		let value = parser(rsx_attributes).parse("attribute={'c'}").unwrap().0;
		assert_eq!(value.to_html(), "attribute='c'");

		let value =
			parser(rsx_attributes).parse("attribute={\"s\"}").unwrap().0;
		assert_eq!(value.to_html(), "attribute=\"s\"");

		// JSX string expressions
		let value = parser(rsx_attributes)
			.parse("attribute={\"bar\"}")
			.unwrap()
			.0;
		assert_eq!(value.to_html(), "attribute=\"bar\"");

		let value =
			parser(rsx_attributes).parse("attribute={'bar'}").unwrap().0;
		assert_eq!(value.to_html(), "attribute='bar'");

		// Boolean literals
		let value = parser(rsx_attributes).parse("attribute=true").unwrap().0;
		assert_eq!(value.to_html(), "attribute=true");

		let value = parser(rsx_attributes).parse("attribute=false").unwrap().0;
		assert_eq!(value.to_html(), "attribute=false");

		// JSX boolean expressions
		let value = parser(rsx_attributes).parse("attribute={true}").unwrap().0;
		assert_eq!(value.to_html(), "attribute=true");

		let value =
			parser(rsx_attributes).parse("attribute={false}").unwrap().0;
		assert_eq!(value.to_html(), "attribute=false");

		// Number literals
		let value = parser(rsx_attributes).parse("attribute=1").unwrap().0;
		assert_eq!(value.to_html(), "attribute=1");

		let value = parser(rsx_attributes).parse("attribute=1.2").unwrap().0;
		assert_eq!(value.to_html(), "attribute=1.2");

		let value = parser(rsx_attributes).parse("attribute=1.2e3").unwrap().0;
		assert_eq!(value.to_html(), "attribute=1200");

		let value = parser(rsx_attributes).parse("attribute=1.2e-3").unwrap().0;
		assert_eq!(value.to_html(), "attribute=0.0012");

		let value = parser(rsx_attributes).parse("attribute=1e3").unwrap().0;
		assert_eq!(value.to_html(), "attribute=1000");

		let value = parser(rsx_attributes).parse("attribute=1e-3").unwrap().0;
		assert_eq!(value.to_html(), "attribute=0.001");

		// Negative number literals
		let value = parser(rsx_attributes).parse("attribute=-1").unwrap().0;
		assert_eq!(value.to_html(), "attribute=-1");

		let value = parser(rsx_attributes).parse("attribute=-1.2").unwrap().0;
		assert_eq!(value.to_html(), "attribute=-1.2");

		let value = parser(rsx_attributes).parse("attribute=-1.2e3").unwrap().0;
		assert_eq!(value.to_html(), "attribute=-1200");

		let value =
			parser(rsx_attributes).parse("attribute=-1.2e-3").unwrap().0;
		assert_eq!(value.to_html(), "attribute=-0.0012");

		let value = parser(rsx_attributes).parse("attribute=-1e3").unwrap().0;
		assert_eq!(value.to_html(), "attribute=-1000");

		let value = parser(rsx_attributes).parse("attribute=-1e-3").unwrap().0;
		assert_eq!(value.to_html(), "attribute=-0.001");

		// JSX number expressions
		let value = parser(rsx_attributes).parse("attribute={1}").unwrap().0;
		assert_eq!(value.to_html(), "attribute=1");

		let value = parser(rsx_attributes).parse("attribute={1.2}").unwrap().0;
		assert_eq!(value.to_html(), "attribute=1.2");

		let value =
			parser(rsx_attributes).parse("attribute={1.2e3}").unwrap().0;
		assert_eq!(value.to_html(), "attribute=1200");

		let value = parser(rsx_attributes)
			.parse("attribute={1.2e-3}")
			.unwrap()
			.0;
		assert_eq!(value.to_html(), "attribute=0.0012");

		let value = parser(rsx_attributes).parse("attribute={1e3}").unwrap().0;
		assert_eq!(value.to_html(), "attribute=1000");

		let value = parser(rsx_attributes).parse("attribute={1e-3}").unwrap().0;
		assert_eq!(value.to_html(), "attribute=0.001");

		// JSX negative number expressions
		let value = parser(rsx_attributes).parse("attribute={-1}").unwrap().0;
		assert_eq!(value.to_html(), "attribute=-1");

		let value = parser(rsx_attributes).parse("attribute={-1.2}").unwrap().0;
		assert_eq!(value.to_html(), "attribute=-1.2");

		let value = parser(rsx_attributes)
			.parse("attribute={-1.2e3}")
			.unwrap()
			.0;
		assert_eq!(value.to_html(), "attribute=-1200");

		let value = parser(rsx_attributes)
			.parse("attribute={-1.2e-3}")
			.unwrap()
			.0;
		assert_eq!(value.to_html(), "attribute=-0.0012");

		let value = parser(rsx_attributes).parse("attribute={-1e3}").unwrap().0;
		assert_eq!(value.to_html(), "attribute=-1000");

		let value =
			parser(rsx_attributes).parse("attribute={-1e-3}").unwrap().0;
		assert_eq!(value.to_html(), "attribute=-0.001");

		// JSX element expressions
		let value = parser(rsx_attributes).parse("attribute=<bar/>").unwrap().0;
		assert_eq!(value.to_html(), "attribute=<bar/>");

		let value = parser(rsx_attributes)
			.parse("attribute={<bar/>}")
			.unwrap()
			.0;
		assert_eq!(value.to_html(), "attribute={<bar/>}");

		// JSX complex expressions
		let value =
			parser(rsx_attributes).parse("attribute={{'c'}}").unwrap().0;
		assert_eq!(value.to_html(), "attribute={{'c'}}");

		let value = parser(rsx_attributes)
			.parse("attribute={{\"s\"}}")
			.unwrap()
			.0;
		assert_eq!(value.to_html(), "attribute={{\"s\"}}");

		let value =
			parser(rsx_attributes).parse("attribute={1+2+3}").unwrap().0;
		assert_eq!(value.to_html(), "attribute={1+2+3}");

		let value = parser(rsx_attributes)
			.parse("attribute={{1+{2+{3}}}}")
			.unwrap()
			.0;
		assert_eq!(value.to_html(), "attribute={{1+{2+{3}}}}");

		// Hyphenated attribute names
		let value = parser(rsx_attributes).parse("foo-bar=\"baz\"").unwrap().0;
		assert_eq!(value.to_html(), "foo-bar=\"baz\"");

		// Namespaced attribute names
		let value = parser(rsx_attributes).parse("foo:bar=\"baz\"").unwrap().0;
		assert_eq!(value.to_html(), "foo:bar=\"baz\"");

		// Complex attribute names
		let value = parser(rsx_attributes)
			.parse("foo-a:bar-b=\"baz\"")
			.unwrap()
			.0;
		assert_eq!(value.to_html(), "foo-a:bar-b=\"baz\"");
	}

	#[test]
	pub fn test_rsx_attributes() {
		assert_eq!(parser(rsx_attributes).parse("").is_err(), true);
		assert_eq!(parser(rsx_attributes).parse(" ").is_err(), true);
		assert_eq!(
			parser(rsx_attributes).parse("foo").unwrap(),
			(RsxAttributes::from(vec![("foo", "true").into()]), "")
		);
		assert_eq!(
			parser(rsx_attributes).parse("foo bar").unwrap(),
			(
				RsxAttributes::from(vec![
					("foo", "true").into(),
					("bar", "true").into()
				]),
				""
			)
		);
		assert_eq!(
			parser(rsx_attributes).parse("foo = 'bar'").unwrap(),
			(RsxAttributes::from(vec![("foo", "bar").into()]), "")
		);
		assert_eq!(
			parser(rsx_attributes).parse("foo='bar' baz").unwrap(),
			(
				RsxAttributes::from(vec![
					("foo", "bar").into(),
					("baz", "true").into()
				]),
				""
			)
		);
		assert_eq!(
			parser(rsx_attributes).parse("foo = 'bar' baz").unwrap(),
			(
				RsxAttributes::from(vec![
					("foo", "bar").into(),
					("baz", "true").into()
				]),
				""
			)
		);
		assert_eq!(
			parser(rsx_attributes).parse("foo='bar' bar='baz'").unwrap(),
			(
				RsxAttributes::from(vec![
					("foo", "bar").into(),
					("bar", "baz").into()
				]),
				""
			)
		);
		assert_eq!(
			parser(rsx_attributes)
				.parse("foo = 'bar' bar='baz'")
				.unwrap(),
			(
				RsxAttributes::from(vec![
					("foo", "bar").into(),
					("bar", "baz").into()
				]),
				""
			)
		);
	}

	#[test]
	pub fn test_rsx_attribute() {
		assert_eq!(parser(rsx_attribute).parse("").is_err(), true);
		assert_eq!(parser(rsx_attribute).parse(" ").is_err(), true);
		assert_eq!(
			parser(rsx_attribute).parse("foo").unwrap(),
			(("foo", "true").into(), "")
		);
		assert_eq!(
			parser(rsx_attribute).parse("foo='bar'").unwrap(),
			(("foo", "bar").into(), "")
		);
		assert_eq!(
			parser(rsx_attribute).parse("foo = 'bar'").unwrap(),
			(("foo", "bar").into(), "")
		);
		assert_eq!(
			parser(rsx_attribute).parse("foo-bar='baz'").unwrap(),
			(("foo-bar", "baz").into(), "")
		);
		assert_eq!(
			parser(rsx_attribute).parse("foo-bar = 'baz'").unwrap(),
			(("foo-bar", "baz").into(), "")
		);
		assert_eq!(
			parser(rsx_attribute).parse("foo:bar='baz'").unwrap(),
			(("foo:bar", "baz").into(), "")
		);
		assert_eq!(
			parser(rsx_attribute).parse("foo:bar = 'baz'").unwrap(),
			(("foo:bar", "baz").into(), "")
		);
		assert_eq!(
			parser(rsx_attribute).parse("foo.bar='baz'").unwrap(),
			(("foo", "true").into(), ".bar='baz'")
		);
		assert_eq!(
			parser(rsx_attribute).parse("foo.bar = 'baz'").unwrap(),
			(("foo", "true").into(), ".bar = 'baz'")
		);
	}

	#[test]
	pub fn test_rsx_attribute_name() {
		assert_eq!(parser(rsx_attribute_complex_name).parse("").is_err(), true);
		assert_eq!(
			parser(rsx_attribute_complex_name).parse(" ").is_err(),
			true
		);
		assert_eq!(
			parser(rsx_attribute_complex_name).parse("foo").unwrap(),
			("foo".into(), "")
		);
		assert_eq!(
			parser(rsx_attribute_complex_name).parse("foo-bar").unwrap(),
			("foo-bar".into(), "")
		);
		assert_eq!(
			parser(rsx_attribute_complex_name).parse("foo:bar").unwrap(),
			("foo:bar".into(), "")
		);
		assert_eq!(
			parser(rsx_attribute_complex_name).parse("foo.bar").unwrap(),
			("foo".into(), ".bar")
		);
	}

	#[test]
	pub fn test_rsx_attribute_value() {
		assert_eq!(parser(rsx_attribute_value).parse("").is_err(), true);
		assert_eq!(parser(rsx_attribute_value).parse(" ").is_err(), true);
		assert_eq!(
			parser(rsx_attribute_value).parse(r#""""#).unwrap(),
			(("", '\'').into(), "")
		);
		assert_eq!(
			parser(rsx_attribute_value).parse(r#"" ""#).unwrap(),
			((" ", '\'').into(), "")
		);
		assert_eq!(
			parser(rsx_attribute_value).parse(r#""foo""#).unwrap(),
			(("foo", '\'').into(), "")
		);
		assert_eq!(
			parser(rsx_attribute_value).parse(r#"'bar'"#).unwrap(),
			(("bar", '"').into(), "")
		);
	}

	#[test]
	pub fn test_rsx_string_characters() {
		assert_eq!(parser(rsx_string_characters).parse("").is_err(), true);
		assert_eq!(parser(rsx_string_characters).parse(" ").is_err(), true);
		assert_eq!(
			parser(rsx_string_characters).parse(r#""""#).unwrap(),
			(("", '\'').into(), "")
		);
		assert_eq!(
			parser(rsx_string_characters).parse(r#"" ""#).unwrap(),
			((" ", '\'').into(), "")
		);
		assert_eq!(
			parser(rsx_string_characters).parse(r#""foo""#).unwrap(),
			(("foo", '\'').into(), "")
		);
		assert_eq!(
			parser(rsx_string_characters).parse(r#"'bar'"#).unwrap(),
			(("bar", '"').into(), "")
		);
		assert_eq!(
			parser(rsx_string_characters).parse(r#""foo'bar""#).unwrap(),
			(("foo'bar", '\'').into(), "")
		);
		assert_eq!(
			parser(rsx_string_characters).parse(r#"'foo"bar'"#).unwrap(),
			((r#"foo"bar"#, '"').into(), "")
		);
		assert_eq!(
			parser(rsx_string_characters)
				.parse(r#""foo\'bar""#)
				.unwrap(),
			(("foo'bar", '\'').into(), "")
		);
		assert_eq!(
			parser(rsx_string_characters)
				.parse(r#"'foo\"bar'"#)
				.unwrap(),
			((r#"foo"bar"#, '"').into(), "")
		);
		assert_eq!(
			parser(rsx_string_characters)
				.parse(r#""foo\"bar""#)
				.unwrap(),
			((r#"foo"bar"#, '\'').into(), "")
		);
		assert_eq!(
			parser(rsx_string_characters)
				.parse(r#"'foo\'bar'"#)
				.unwrap(),
			(("foo'bar", '"').into(), "")
		);
		assert_eq!(
			parser(rsx_string_characters).parse(r#""foo"bar""#).unwrap(),
			(("foo", '\'').into(), "bar\"")
		);
		assert_eq!(
			parser(rsx_string_characters).parse(r#"'foo'bar'"#).unwrap(),
			(("foo", '"').into(), "bar\'")
		);
	}

	#[test]
	pub fn test_rsx_bracketed_string_characters() {
		assert_eq!(
			parser(rsx_bracketed_string_characters).parse("").is_err(),
			true
		);
		assert_eq!(
			parser(rsx_bracketed_string_characters).parse(" ").is_err(),
			true
		);
		assert_eq!(
			parser(rsx_bracketed_string_characters)
				.parse(r#"{""}"#)
				.unwrap(),
			(("", '\'').into(), "")
		);
		assert_eq!(
			parser(rsx_bracketed_string_characters)
				.parse(r#"{" "}"#)
				.unwrap(),
			((" ", '\'').into(), "")
		);
		assert_eq!(
			parser(rsx_bracketed_string_characters)
				.parse(r#"{"{{{}}}"}"#)
				.unwrap(),
			(("{{{}}}", '\'').into(), "")
		);
		assert_eq!(
			parser(rsx_bracketed_string_characters)
				.parse(r#"{"foo"}"#)
				.unwrap(),
			(("foo", '\'').into(), "")
		);
		assert_eq!(
			parser(rsx_bracketed_string_characters)
				.parse(r#"{'bar'}"#)
				.unwrap(),
			(("bar", '"').into(), "")
		);
		assert_eq!(
			parser(rsx_bracketed_string_characters)
				.parse(r#"{"foo'bar"}"#)
				.unwrap(),
			(("foo'bar", '\'').into(), "")
		);
		assert_eq!(
			parser(rsx_bracketed_string_characters)
				.parse(r#"{'foo"bar'}"#)
				.unwrap(),
			((r#"foo"bar"#, '"').into(), "")
		);
		assert_eq!(
			parser(rsx_bracketed_string_characters)
				.parse(r#"{"foo\'bar"}"#)
				.unwrap(),
			(("foo'bar", '\'').into(), "")
		);
		assert_eq!(
			parser(rsx_bracketed_string_characters)
				.parse(r#"{'foo\"bar'}"#)
				.unwrap(),
			((r#"foo"bar"#, '"').into(), "")
		);
		assert_eq!(
			parser(rsx_bracketed_string_characters)
				.parse(r#"{"foo\"bar"}"#)
				.unwrap(),
			((r#"foo"bar"#, '\'').into(), "")
		);
		assert_eq!(
			parser(rsx_bracketed_string_characters)
				.parse(r#"{'foo\'bar'}"#)
				.unwrap(),
			(("foo'bar", '"').into(), "")
		);
		assert_eq!(
			parser(rsx_bracketed_string_characters)
				.parse(r#"{"foo"bar"}"#)
				.is_err(),
			true
		);
		assert_eq!(
			parser(rsx_bracketed_string_characters)
				.parse(r#"{'foo'bar'}"#)
				.is_err(),
			true
		);
	}
}
