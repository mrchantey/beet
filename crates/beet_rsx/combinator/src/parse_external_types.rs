use crate::prelude::*;
use std::iter::FromIterator;

#[derive(Debug, PartialEq)]
pub enum RsxRawCodeFragment {
	Empty,
	Token(char),
	Tokens(String),
	Element(RsxElement),
	ParsedExpression(RsxParsedExpression),
}


/// A vec of either tokens or elements, in the order they were parsed.
/// Adjacent tokens are merged into a single string.
#[derive(Debug, Default, PartialEq)]
pub struct RsxParsedExpression(pub(crate) Vec<RsxTokensOrElement>);

impl std::ops::Deref for RsxParsedExpression {
	type Target = Vec<RsxTokensOrElement>;
	fn deref(&self) -> &Self::Target { &self.0 }
}

impl RsxParsedExpression {
	pub fn inner(self) -> Vec<RsxTokensOrElement> { self.0 }

	pub fn push_element(&mut self, element: RsxElement) {
		self.0.push(RsxTokensOrElement::Element(element));
	}

	pub fn push_str(&mut self, s: &str) {
		match self.0.last_mut() {
			Some(RsxTokensOrElement::Tokens(last)) => {
				last.push_str(s);
			}
			_ => {
				self.0.push(s.into());
			}
		}
	}
	pub fn push_char(&mut self, c: char) {
		match self.0.last_mut() {
			Some(RsxTokensOrElement::Tokens(last)) => {
				last.push(c);
			}
			_ => {
				self.0.push(c.into());
			}
		}
	}
}

#[derive(Debug, PartialEq)]
pub enum RsxTokensOrElement {
	Tokens(String),
	Element(RsxElement),
}

impl Into<RsxTokensOrElement> for &str {
	fn into(self) -> RsxTokensOrElement {
		RsxTokensOrElement::Tokens(self.to_string())
	}
}
impl Into<RsxTokensOrElement> for String {
	fn into(self) -> RsxTokensOrElement { RsxTokensOrElement::Tokens(self) }
}
impl Into<RsxTokensOrElement> for char {
	fn into(self) -> RsxTokensOrElement {
		RsxTokensOrElement::Tokens(self.to_string())
	}
}
impl Into<RsxTokensOrElement> for RsxElement {
	fn into(self) -> RsxTokensOrElement { RsxTokensOrElement::Element(self) }
}

impl RsxParsedExpression {
	pub fn index_to_placeholder(index: usize) -> String {
		format!("/* rsx:{} */", index)
	}
}


impl FromIterator<RsxRawCodeFragment> for RsxParsedExpression {
	fn from_iter<I: IntoIterator<Item = RsxRawCodeFragment>>(
		fragments: I,
	) -> Self {
		let mut expression = RsxParsedExpression::default();
		fragments.into_iter().for_each(|fragment| match fragment {
			RsxRawCodeFragment::Empty => {}
			RsxRawCodeFragment::Token(c) => expression.push_char(c),
			RsxRawCodeFragment::Tokens(s) => expression.push_str(&s),
			RsxRawCodeFragment::Element(element) => {
				expression.push_element(element);
			}
			RsxRawCodeFragment::ParsedExpression(other) => {
				expression.push_char('{');
				let mut other = other.0.into_iter();
				if let Some(RsxTokensOrElement::Tokens(first)) = other.next() {
					expression.push_str(&first);
				}
				expression.0.extend(other);
				expression.push_char('}');
			}
		});
		// i *think* this is safe to do, trying to imagine adjacent ParsedExpressions
		// that depend on whitespace?
		// if let Some(RsxTokensOrElement::Tokens(last)) = expression.0.last_mut()
		// {
		// 	if last.trim().is_empty() {
		// 		expression.0.pop();
		// 	}
		// }
		expression
	}
}
