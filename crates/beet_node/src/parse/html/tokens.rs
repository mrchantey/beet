//! Intermediate token types produced by the HTML parser combinators.

use std::fmt;

/// A single parsed HTML token, produced by the winnow combinators
/// and consumed by the entity differ.
#[derive(Debug, Clone, PartialEq)]
pub enum HtmlToken {
	/// `<!DOCTYPE ...>`
	Doctype(String),
	/// An opening tag with name, attributes, and whether it is self-closing.
	OpenTag {
		name: String,
		attributes: Vec<HtmlAttribute>,
		self_closing: bool,
	},
	/// A closing tag `</name>`.
	CloseTag(String),
	/// Text content between tags, whitespace preserved.
	Text(String),
	/// `<!-- content -->`
	Comment(String),
	/// `{expression}` when `parse_expressions` is enabled.
	Expression(String),
}

/// A single attribute parsed from an opening tag.
#[derive(Debug, Clone, PartialEq)]
pub struct HtmlAttribute {
	/// Attribute name, empty for keyless expression attributes.
	pub key: String,
	/// Attribute value, `None` for boolean attributes like `disabled`.
	pub value: Option<String>,
	/// Whether this attribute is an expression `{foo}` rather than a
	/// standard `key="value"` pair.
	pub expression: bool,
}

impl HtmlAttribute {
	/// Creates a standard `key="value"` attribute.
	pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
		Self {
			key: key.into(),
			value: Some(value.into()),
			expression: false,
		}
	}

	/// Creates a boolean attribute with no value, ie `disabled`.
	pub fn boolean(key: impl Into<String>) -> Self {
		Self {
			key: key.into(),
			value: None,
			expression: false,
		}
	}

	/// Creates a keyless expression attribute `{expr}`.
	pub fn expression(expr: impl Into<String>) -> Self {
		Self {
			key: String::new(),
			value: Some(expr.into()),
			expression: true,
		}
	}

	/// Creates a keyed expression attribute, ie `key={expr}`.
	pub fn keyed_expression(
		key: impl Into<String>,
		expr: impl Into<String>,
	) -> Self {
		Self {
			key: key.into(),
			value: Some(expr.into()),
			expression: true,
		}
	}
}

impl fmt::Display for HtmlToken {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			HtmlToken::Doctype(val) => write!(fmt, "<!DOCTYPE {val}>"),
			HtmlToken::OpenTag {
				name,
				attributes,
				self_closing,
			} => {
				write!(fmt, "<{name}")?;
				for attr in attributes {
					write!(fmt, " {attr}")?;
				}
				if *self_closing {
					write!(fmt, " />")?;
				} else {
					write!(fmt, ">")?;
				}
				Ok(())
			}
			HtmlToken::CloseTag(name) => write!(fmt, "</{name}>"),
			HtmlToken::Text(text) => write!(fmt, "{text}"),
			HtmlToken::Comment(text) => write!(fmt, "<!--{text}-->"),
			HtmlToken::Expression(expr) => write!(fmt, "{{{expr}}}"),
		}
	}
}

impl fmt::Display for HtmlAttribute {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		if self.expression {
			if self.key.is_empty() {
				// keyless expression: {expr}
				if let Some(val) = &self.value {
					write!(fmt, "{{{val}}}")
				} else {
					write!(fmt, "{{}}")
				}
			} else {
				// keyed expression: key={expr}
				if let Some(val) = &self.value {
					write!(fmt, "{}={{{val}}}", self.key)
				} else {
					write!(fmt, "{}", self.key)
				}
			}
		} else if let Some(val) = &self.value {
			write!(fmt, "{}=\"{val}\"", self.key)
		} else {
			// boolean attribute
			write!(fmt, "{}", self.key)
		}
	}
}


#[cfg(test)]
mod test {
	use super::*;
	use beet_core::prelude::*;

	#[test]
	fn display_open_tag_simple() {
		HtmlToken::OpenTag {
			name: "div".into(),
			attributes: vec![],
			self_closing: false,
		}
		.to_string()
		.xpect_eq("<div>".to_string());
	}

	#[test]
	fn display_open_tag_with_attrs() {
		HtmlToken::OpenTag {
			name: "input".into(),
			attributes: vec![
				HtmlAttribute::new("type", "text"),
				HtmlAttribute::boolean("disabled"),
			],
			self_closing: true,
		}
		.to_string()
		.xpect_eq("<input type=\"text\" disabled />".to_string());
	}

	#[test]
	fn display_close_tag() {
		HtmlToken::CloseTag("div".into())
			.to_string()
			.xpect_eq("</div>".to_string());
	}

	#[test]
	fn display_text() {
		HtmlToken::Text("hello world".into())
			.to_string()
			.xpect_eq("hello world".to_string());
	}

	#[test]
	fn display_comment() {
		HtmlToken::Comment(" a comment ".into())
			.to_string()
			.xpect_eq("<!-- a comment -->".to_string());
	}

	#[test]
	fn display_expression() {
		HtmlToken::Expression("foo.bar".into())
			.to_string()
			.xpect_eq("{foo.bar}".to_string());
	}

	#[test]
	fn display_keyless_expression_attr() {
		HtmlAttribute::expression("foo")
			.to_string()
			.xpect_eq("{foo}".to_string());
	}

	#[test]
	fn display_keyed_expression_attr() {
		HtmlAttribute::keyed_expression("onclick", "handler")
			.to_string()
			.xpect_eq("onclick={handler}".to_string());
	}
}
