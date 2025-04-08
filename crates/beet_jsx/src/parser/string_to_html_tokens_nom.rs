use crate::prelude::*;
use anyhow::Result;
use nom::IResult;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::bytes::complete::take_until;
use nom::bytes::complete::take_while1;
use nom::character::complete::char;
use nom::character::complete::multispace0;
use nom::character::complete::space0;
use nom::combinator::map;
use nom::combinator::opt;
use nom::combinator::recognize;
use nom::multi::many0;
use nom::multi::many1;
use nom::sequence::delimited;
use nom::sequence::preceded;
use nom::sequence::tuple;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use std::fmt::Formatter;
use sweet::prelude::*;
use syn::Expr;
use syn::LitStr;

#[derive(Debug, Default)]
pub struct StringToHtmlTokens;

impl Pipeline<String, Result<HtmlTokens>> for StringToHtmlTokens {
	fn apply(self, input: String) -> Result<HtmlTokens> {
		let (remaining, node) = parse_html(&input).map_err(|e| {
			anyhow::anyhow!("Failed to parse HTML: {}", e.to_string())
		})?;
		if !remaining.is_empty() {
			return Err(anyhow::anyhow!(
				"Unparsed input remaining: {}",
				remaining
			));
		}
		Ok(node.into())
	}
}



/// absolute bare bones html parsing
#[derive(Debug, PartialEq)]
enum HtmlNode {
	Element {
		name: String,
		attributes: Vec<HtmlAttribute>,
		children: Vec<HtmlNode>,
		self_closing: bool,
	},
	Text(String),
}

impl Into<HtmlTokens> for HtmlNode {
	fn into(self) -> HtmlTokens {
		match self {
			HtmlNode::Element {
				name,
				attributes,
				children,
				self_closing,
			} => HtmlTokens::Element {
				self_closing,
				component: RsxNodeTokens {
					tag: NameExpr::LitStr(
						LitStr::new(&name, Span::call_site()).into(),
					),
					tokens: TokenStream::new(),
					attributes: attributes
						.into_iter()
						.map(Into::into)
						.collect(),
					directives: Vec::new(),
				},
				children: Box::new(HtmlTokens::collapse(
					children.into_iter().map(Into::into).collect(),
				)),
			},
			HtmlNode::Text(text) => HtmlTokens::Text {
				value: LitStr::new(&text, Span::call_site()).into(),
			},
		}
	}
}

impl std::fmt::Display for HtmlNode {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			HtmlNode::Element {
				name,
				attributes,
				self_closing,
				children,
			} => {
				write!(f, "<{}", name)?;
				for attr in attributes {
					write!(f, " {}", attr)?;
				}
				if *self_closing {
					write!(f, "/>")
				} else {
					write!(f, ">")?;
					for child in children {
						write!(f, "{}", child)?;
					}
					write!(f, "</{}>", name)
				}
			}
			HtmlNode::Text(text) => write!(f, "{}", text),
		}
	}
}


#[derive(Debug, PartialEq)]
enum HtmlAttribute {
	Key { key: String },
	KeyValue { key: String, value: String },
}

impl std::fmt::Display for HtmlAttribute {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			HtmlAttribute::Key { key } => write!(f, "{}", key),
			HtmlAttribute::KeyValue { key, value } => {
				write!(f, "{}=\"{}\"", key, value)
			}
		}
	}
}

impl Into<RsxAttributeTokens> for HtmlAttribute {
	fn into(self) -> RsxAttributeTokens {
		match self {
			HtmlAttribute::Key { key } => RsxAttributeTokens::Key {
				key: NameExpr::LitStr(
					LitStr::new(&key, Span::call_site()).into(),
				),
			},
			HtmlAttribute::KeyValue { key, value } => {
				RsxAttributeTokens::KeyValue {
					key: NameExpr::LitStr(
						LitStr::new(&key, Span::call_site()).into(),
					),
					value: syn::parse_str::<Expr>(&value).unwrap().into(),
				}
			}
		}
	}
}

// Parser for HTML attribute names
fn attribute_name(input: &str) -> IResult<&str, &str> {
	take_while1(|c: char| {
		c.is_alphanumeric() || c == '-' || c == '_' || c == ':'
	})(input)
}

// Parser for HTML attribute values
fn attribute_value(input: &str) -> IResult<&str, &str> {
	alt((
		delimited(char('"'), take_until("\""), char('"')),
		delimited(char('\''), take_until("'"), char('\'')),
	))(input)
}

// Parser for HTML attributes
fn attribute(input: &str) -> IResult<&str, HtmlAttribute> {
	let (input, name) = attribute_name(input)?;
	let (input, value) = opt(preceded(
		tuple((space0, char('='), space0)),
		attribute_value,
	))(input)?;

	let key = name.to_string();
	let attr = if let Some(value) = value {
		HtmlAttribute::KeyValue {
			key,
			value: value.to_string(),
		}
	} else {
		HtmlAttribute::Key { key }
	};

	Ok((input, attr))
}

// Parser for HTML attributes list
fn attributes(input: &str) -> IResult<&str, Vec<HtmlAttribute>> {
	many0(preceded(space0, attribute))(input)
}

// Parser for HTML opening tags
fn open_tag(input: &str) -> IResult<&str, (String, Vec<HtmlAttribute>)> {
	let (input, _) = char('<')(input)?;
	let (input, name) = attribute_name(input)?;
	let (input, attrs) = attributes(input)?;
	let (input, _) = tuple((space0, char('>')))(input)?;

	Ok((input, (name.to_string(), attrs)))
}

// Parser for self-closing tags
fn self_closing_tag(
	input: &str,
) -> IResult<&str, (String, Vec<HtmlAttribute>)> {
	let (input, _) = char('<')(input)?;
	let (input, name) = attribute_name(input)?;
	let (input, attrs) = attributes(input)?;
	let (input, _) = tuple((space0, tag("/>")))(input)?;

	Ok((input, (name.to_string(), attrs)))
}

// Parser for HTML closing tags
fn close_tag(input: &str) -> IResult<&str, String> {
	let (input, _) = tag("</")(input)?;
	let (input, name) = attribute_name(input)?;
	let (input, _) = tuple((space0, char('>')))(input)?;

	Ok((input, name.to_string()))
}

// Parser for HTML text nodes
fn text(input: &str) -> IResult<&str, String> {
	let (input, content) =
		recognize(many1(take_while1(|c: char| c != '<')))(input)?;

	Ok((
		input,
		content.replace("\n", "").replace("\t", "").to_string(),
	))
}

// Main parser for HTML elements
fn element(input: &str) -> IResult<&str, HtmlNode> {
	alt((
		// Self-closing tag
		map(self_closing_tag, |(name, attributes)| HtmlNode::Element {
			name,
			attributes,
			self_closing: true,
			children: vec![],
		}),
		// Regular tag with children
		map(
			tuple((
				open_tag,
				many0(alt((map(text, HtmlNode::Text), element))),
				close_tag,
			)),
			|((name, attributes), children, _close_name)| {
				// In a real parser, we'd want to validate that the closing tag matches
				// the opening tag, but we'll skip that for simplicity
				HtmlNode::Element {
					name,
					attributes,
					self_closing: false,
					children,
				}
			},
		),
		// Text node
		map(text, HtmlNode::Text),
	))(input)
}

// Parse an HTML document
fn parse_html(input: &str) -> IResult<&str, HtmlNode> {
	delimited(multispace0, element, multispace0)(input)
}

#[cfg(test)]
mod test {
	// use crate::prelude::*;
	use super::*;
	// use sweet::prelude::*;

	#[test]
	fn works() {
		let html = r#"<div class="container">
		<h1>Hello World</h1>
		<p id="intro" data-value="42">
				This is a <strong>simple</strong> HTML parser.
		</p>
		<img src="image.jpg" alt="An image" />
</div>"#;

		let node = parse_html(html.into()).unwrap().1;
		expect(node.to_string()).to_be("<div class=\"container\"><h1>Hello World</h1><p id=\"intro\" data-value=\"42\">This is a <strong>simple</strong> HTML parser.</p><img src=\"image.jpg\" alt=\"An image\"/></div>");
	}
}
