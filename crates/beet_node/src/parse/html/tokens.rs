//! Intermediate token types produced by the HTML parser combinators.
//!
//! These types borrow slices from the input string for zero-copy parsing.
//! Owned strings are only created when building ECS components.

use std::fmt;

/// A single parsed HTML token, produced by the winnow combinators
/// and consumed by the entity differ.
///
/// Borrows string slices from the input for zero-copy parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum HtmlToken<'a> {
	/// `<!DOCTYPE ...>`
	Doctype(&'a str),
	/// An opening tag with name, attributes, and whether it is self-closing.
	///
	/// `source` is the full opening tag text from `<` to `>` (inclusive),
	/// used for [`FileSpan`] tracking.
	OpenTag {
		name: &'a str,
		attributes: Vec<HtmlAttribute<'a>>,
		self_closing: bool,
		/// The full source text of the opening tag, ie `<div class="foo">`.
		source: &'a str,
	},
	/// A closing tag `</name>`.
	CloseTag(&'a str),
	/// Text content between tags, whitespace preserved.
	Text(&'a str),
	/// `<!-- content -->`
	Comment(&'a str),
	/// `{expression}` when `parse_expressions` is enabled.
	Expression(&'a str),
}

/// A single attribute parsed from an opening tag.
///
/// Borrows string slices from the input for zero-copy parsing.
#[derive(Debug, Clone, PartialEq)]
pub struct HtmlAttribute<'a> {
	/// Attribute name, empty for keyless expression attributes.
	pub key: &'a str,
	/// Attribute value, `None` for boolean attributes like `disabled`.
	pub value: Option<&'a str>,
	/// Whether this attribute is an expression `{foo}` rather than a
	/// standard `key="value"` pair.
	pub expression: bool,
}

impl<'a> HtmlAttribute<'a> {
	/// Creates a standard `key="value"` attribute.
	pub fn new(key: &'a str, value: &'a str) -> Self {
		Self {
			key,
			value: Some(value),
			expression: false,
		}
	}

	/// Creates a boolean attribute with no value, ie `disabled`.
	pub fn boolean(key: &'a str) -> Self {
		Self {
			key,
			value: None,
			expression: false,
		}
	}

	/// Creates a keyless expression attribute `{expr}`.
	pub fn expression(expr: &'a str) -> Self {
		Self {
			key: "",
			value: Some(expr),
			expression: true,
		}
	}

	/// Creates a keyed expression attribute, ie `key={expr}`.
	pub fn keyed_expression(key: &'a str, expr: &'a str) -> Self {
		Self {
			key,
			value: Some(expr),
			expression: true,
		}
	}

	/// Returns the effective key for this attribute.
	///
	/// For keyless expressions, returns the expression content as key.
	pub fn effective_key(&self) -> &str {
		if self.expression && self.key.is_empty() {
			self.value.unwrap_or("")
		} else {
			self.key
		}
	}
}

impl fmt::Display for HtmlToken<'_> {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			HtmlToken::Doctype(val) => write!(fmt, "<!DOCTYPE {val}>"),
			HtmlToken::OpenTag {
				name,
				attributes,
				self_closing,
				..
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

impl fmt::Display for HtmlAttribute<'_> {
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
			name: "div",
			attributes: vec![],
			self_closing: false,
			source: "<div>",
		}
		.to_string()
		.xpect_eq("<div>".to_string());
	}

	#[test]
	fn display_open_tag_with_attrs() {
		HtmlToken::OpenTag {
			name: "input",
			attributes: vec![
				HtmlAttribute::new("type", "text"),
				HtmlAttribute::boolean("disabled"),
			],
			self_closing: true,
			source: "<input type=\"text\" disabled />",
		}
		.to_string()
		.xpect_eq("<input type=\"text\" disabled />".to_string());
	}

	#[test]
	fn display_close_tag() {
		HtmlToken::CloseTag("div")
			.to_string()
			.xpect_eq("</div>".to_string());
	}

	#[test]
	fn display_text() {
		HtmlToken::Text("hello world")
			.to_string()
			.xpect_eq("hello world".to_string());
	}

	#[test]
	fn display_comment() {
		HtmlToken::Comment(" a comment ")
			.to_string()
			.xpect_eq("<!-- a comment -->".to_string());
	}

	#[test]
	fn display_expression() {
		HtmlToken::Expression("foo.bar")
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

	#[test]
	fn effective_key_standard() {
		HtmlAttribute::new("class", "foo")
			.effective_key()
			.xpect_eq("class");
	}

	#[test]
	fn effective_key_boolean() {
		HtmlAttribute::boolean("disabled")
			.effective_key()
			.xpect_eq("disabled");
	}

	#[test]
	fn effective_key_keyless_expression() {
		HtmlAttribute::expression("foo")
			.effective_key()
			.xpect_eq("foo");
	}

	#[test]
	fn effective_key_keyed_expression() {
		HtmlAttribute::keyed_expression("onclick", "handler")
			.effective_key()
			.xpect_eq("onclick");
	}
}
