use crate::prelude::*;


/// Parse the given source into a html string with
/// no transformations applied, ie <div foo=true/> will not
/// be quoted into a "true" html attribute.
pub trait ToHtml {
	fn to_html(&self) -> String;
}


impl ToHtml for RsxElement {
	fn to_html(&self) -> String {
		match self {
			RsxElement::SelfClosing(element) => element.to_html(),
			RsxElement::Normal(element) => element.to_html(),
		}
	}
}

impl ToHtml for RsxSelfClosingElement {
	fn to_html(&self) -> String {
		let mut result = format!("<");
		result.push_str(&self.0.to_html());
		if !self.1.0.is_empty() {
			result.push(' ');
		}
		result.push_str(&self.1.to_html());
		result.push_str("/>");
		result
	}
}


impl ToHtml for RsxNormalElement {
	fn to_html(&self) -> String {
		let mut result = format!("<");
		result.push_str(&self.0.to_html());
		if !self.1.0.is_empty() {
			result.push(' ');
		}
		result.push_str(&self.1.to_html());
		result.push_str(">");
		result.push_str(&self.2.to_html());
		result.push_str(&format!("</{}>", self.0));
		result
	}
}
impl ToHtml for RsxChildren {
	fn to_html(&self) -> String {
		self.0
			.iter()
			.map(|child| child.to_html())
			.collect::<Vec<_>>()
			.join("")
	}
}

impl ToHtml for RsxChild {
	fn to_html(&self) -> String {
		match self {
			RsxChild::Text(text) => text.to_html(),
			RsxChild::Element(element) => element.to_html(),
			RsxChild::CodeBlock(code_block) => code_block.to_html(),
		}
	}
}

impl ToHtml for RsxText {
	fn to_html(&self) -> String { self.0.clone() }
}

impl ToHtml for RsxElementName {
	fn to_html(&self) -> String { self.to_string() }
}

impl ToHtml for RsxAttributes {
	fn to_html(&self) -> String {
		self.0
			.iter()
			.map(|attr| attr.to_html())
			.collect::<Vec<_>>()
			.join(" ")
	}
}

impl ToHtml for RsxAttribute {
	fn to_html(&self) -> String {
		match self {
			RsxAttribute::Named(name, value) => match value {
				RsxAttributeValue::Default => name.to_string(),
				_ => {
					format!("{}={}", name.to_string(), value.to_html())
				}
			},
			RsxAttribute::Spread(value) => value.to_html(),
		}
	}
}

impl ToHtml for RsxAttributeValue {
	fn to_html(&self) -> String {
		match self {
			RsxAttributeValue::Default => panic!(
				"This should not be reachable as the equals sign will be empty"
			),
			RsxAttributeValue::Boolean(value) => value.to_html(),
			RsxAttributeValue::Number(value) => value.to_html(),
			RsxAttributeValue::Str(value) => value.to_html(),
			RsxAttributeValue::Element(element) => element.to_html(),
			RsxAttributeValue::CodeBlock(value) => value.to_html(),
		}
	}
}

impl ToHtml for RsxAttributeBoolean {
	fn to_html(&self) -> String { self.0.to_string() }
}

impl ToHtml for RsxAttributeNumber {
	fn to_html(&self) -> String { self.0.to_string() }
}

impl ToHtml for RsxAttributeString {
	fn to_html(&self) -> String {
		match self {
			RsxAttributeString::SingleQuoted(value) => format!("'{}'", value.0),
			RsxAttributeString::DoubleQuoted(value) => {
				format!("\"{}\"", value.0)
			}
		}
	}
}

impl ToHtml for RsxParsedExpression {
	fn to_html(&self) -> String {
		let mut result = String::new();

		for item in self.iter() {
			match item {
				RsxTokensOrElement::Tokens(str) => {
					result.push_str(&str);
				}
				RsxTokensOrElement::Element(el) => {
					result.push_str(&el.to_html());
				}
			}
		}

		format!("{{{result}}}")
	}
}

// use RsxParsedExpression instead
// impl ToHtml for RsxRawCodeFragment {
// 	fn to_html(&self) -> String {
// 		match self {
// 			RsxRawCodeFragment::Empty => String::new(),
// 			RsxRawCodeFragment::Token(c) => c.to_string(),
// 			RsxRawCodeFragment::Tokens(s) => s.clone(),
// 			RsxRawCodeFragment::Element(element) => element.to_html(),
// 			RsxRawCodeFragment::ParsedExpression(expression) => {
// 				expression.to_html()
// 			}
// 		}
// 	}
// }


impl<T: ToHtml> ToHtml for Vec<T> {
	fn to_html(&self) -> String {
		self.iter()
			.map(|item| item.to_html())
			.collect::<Vec<_>>()
			.join("")
	}
}



#[cfg(test)]
mod test {
	// use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() { expect(true).to_be_true(); }
}
