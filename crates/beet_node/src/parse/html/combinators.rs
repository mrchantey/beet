//! Winnow parser combinators for HTML tokenization.
//!
//! All combinators operate on `&str` input and produce [`HtmlToken`] or
//! sub-components thereof. The parsers are designed to be composed by
//! the top-level [`parse_document`] entry point.

use super::tokens::*;
use winnow::ModalResult;
use winnow::Parser;
use winnow::error::ContextError;
use winnow::error::ErrMode;
use winnow::token::any;
use winnow::token::take_till;
use winnow::token::take_until;
use winnow::token::take_while;

/// Parser configuration passed through combinators to control behavior.
#[derive(Debug, Clone)]
pub struct ParseConfig {
	/// Enable `{expr}` expression parsing in content and attributes.
	pub parse_expressions: bool,
	/// Enable `{{expr}}` double-escaped expressions in raw text elements.
	pub parse_raw_text_expressions: bool,
	/// Element names whose content is raw text, ie `<script>`, `<style>`.
	pub raw_text_elements: Vec<String>,
	/// Element names whose content is raw character data, ie `<textarea>`, `<title>`.
	pub raw_character_data_elements: Vec<String>,
}

impl Default for ParseConfig {
	fn default() -> Self {
		Self {
			parse_expressions: false,
			parse_raw_text_expressions: false,
			raw_text_elements: vec!["script".into(), "style".into()],
			raw_character_data_elements: vec![
				"textarea".into(),
				"title".into(),
			],
		}
	}
}

impl ParseConfig {
	/// Returns whether the given element name is a raw text or raw character data element.
	pub fn is_raw_text_element(&self, name: &str) -> bool {
		let lower = name.to_ascii_lowercase();
		self.raw_text_elements.iter().any(|el| *el == lower)
			|| self
				.raw_character_data_elements
				.iter()
				.any(|el| *el == lower)
	}
}

/// Parse a complete HTML document into a sequence of tokens.
pub fn parse_document(
	input: &str,
	config: &ParseConfig,
) -> Result<Vec<HtmlToken>, String> {
	let mut remaining = input;
	let mut tokens = Vec::new();

	while !remaining.is_empty() {
		match parse_node(config).parse_next(&mut remaining) {
			Ok(token) => tokens.push(token),
			Err(err) => {
				return Err(format!(
					"parse error at byte {}: {err}",
					input.len() - remaining.len()
				));
			}
		}
	}

	Ok(tokens)
}

/// Parse a single HTML node, dispatching to the appropriate sub-parser.
fn parse_node<'input>(
	config: &ParseConfig,
) -> impl Parser<&'input str, HtmlToken, ErrMode<ContextError>> + '_ {
	move |input: &mut &'input str| {
		if input.starts_with("<!--") {
			return parse_comment.parse_next(input);
		}
		if input.starts_with("<!") {
			return parse_doctype.parse_next(input);
		}
		if input.starts_with("</") {
			return parse_close_tag.parse_next(input);
		}
		if input.starts_with('<') {
			return parse_open_tag.parse_next(input);
		}
		if config.parse_expressions && input.starts_with('{') {
			return parse_expression.parse_next(input);
		}
		parse_text(config.parse_expressions).parse_next(input)
	}
}

/// Parse an HTML comment: `<!-- content -->`.
fn parse_comment(input: &mut &str) -> ModalResult<HtmlToken, ContextError> {
	let _ = "<!--".parse_next(input)?;
	let content = take_until(0.., "-->").parse_next(input)?;
	let _ = "-->".parse_next(input)?;
	Ok(HtmlToken::Comment(content.to_string()))
}

/// Parse a doctype declaration: `<!DOCTYPE ...>`.
fn parse_doctype(input: &mut &str) -> ModalResult<HtmlToken, ContextError> {
	let _ = "<!".parse_next(input)?;
	let content = take_till(1.., '>').parse_next(input)?;
	let _ = '>'.parse_next(input)?;
	Ok(HtmlToken::Doctype(content.to_string()))
}

/// Parse a closing tag: `</name>`.
fn parse_close_tag(input: &mut &str) -> ModalResult<HtmlToken, ContextError> {
	let _ = "</".parse_next(input)?;
	let _ = take_while(0.., |ch: char| ch.is_ascii_whitespace())
		.parse_next(input)?;
	let name = parse_tag_name(input)?;
	let _ = take_while(0.., |ch: char| ch.is_ascii_whitespace())
		.parse_next(input)?;
	let _ = '>'.parse_next(input)?;
	Ok(HtmlToken::CloseTag(name))
}

/// Parse an opening tag: `<name attr="val" ...>` or self-closing `<name />`.
fn parse_open_tag(input: &mut &str) -> ModalResult<HtmlToken, ContextError> {
	let _ = '<'.parse_next(input)?;
	let name = parse_tag_name(input)?;

	let mut attributes = Vec::new();

	loop {
		// consume whitespace
		let _ = take_while(0.., |ch: char| ch.is_ascii_whitespace())
			.parse_next(input)?;

		// check for end of tag
		if input.starts_with("/>") {
			let _ = "/>".parse_next(input)?;
			return Ok(HtmlToken::OpenTag {
				name,
				attributes,
				self_closing: true,
			});
		}
		if input.starts_with('>') {
			let _ = '>'.parse_next(input)?;
			return Ok(HtmlToken::OpenTag {
				name,
				attributes,
				self_closing: false,
			});
		}

		if input.is_empty() {
			return Err(ErrMode::Cut(ContextError::new()));
		}

		// parse an attribute
		let attr = parse_attribute(input)?;
		attributes.push(attr);
	}
}

/// Parse a valid HTML tag name: starts with ascii alpha, followed by
/// alphanumeric or hyphens (for custom elements).
fn parse_tag_name(input: &mut &str) -> ModalResult<String, ContextError> {
	let first = winnow::token::one_of(|ch: char| ch.is_ascii_alphabetic())
		.parse_next(input)?;
	let rest = take_while(0.., |ch: char| {
		ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.'
	})
	.parse_next(input)?;
	Ok(format!("{first}{rest}"))
}

/// Parse a single attribute from inside an opening tag.
///
/// Supports:
/// - `key="value"` or `key='value'`
/// - `key=unquoted`
/// - `key` (boolean, no value)
/// - `{expr}` (keyless expression)
/// - `key={expr}` (keyed expression)
fn parse_attribute(
	input: &mut &str,
) -> ModalResult<HtmlAttribute, ContextError> {
	// keyless expression attribute: {expr}
	if input.starts_with('{') {
		let expr = parse_brace_content(input)?;
		return Ok(HtmlAttribute::expression(expr));
	}

	// parse attribute name
	let key = parse_attribute_name(input)?;

	// consume whitespace around =
	let _ = take_while(0.., |ch: char| ch.is_ascii_whitespace())
		.parse_next(input)?;

	// check for = sign
	if !input.starts_with('=') {
		// boolean attribute
		return Ok(HtmlAttribute::boolean(key));
	}
	let _ = '='.parse_next(input)?;

	let _ = take_while(0.., |ch: char| ch.is_ascii_whitespace())
		.parse_next(input)?;

	// expression value: key={expr}
	if input.starts_with('{') {
		let expr = parse_brace_content(input)?;
		return Ok(HtmlAttribute::keyed_expression(key, expr));
	}

	// quoted or unquoted value
	let value = parse_attribute_value(input)?;
	Ok(HtmlAttribute::new(key, value))
}

/// Parse an attribute name: alphanumeric, hyphens, underscores, colons, periods.
fn parse_attribute_name(input: &mut &str) -> ModalResult<String, ContextError> {
	let name =
		take_while(1.., |ch: char| {
			ch.is_ascii_alphanumeric()
				|| ch == '-' || ch == '_'
				|| ch == ':' || ch == '.'
				|| ch == '@'
		})
		.parse_next(input)?;
	Ok(name.to_string())
}

/// Parse an attribute value: double-quoted, single-quoted, or unquoted.
fn parse_attribute_value(
	input: &mut &str,
) -> ModalResult<String, ContextError> {
	if input.starts_with('"') {
		// double-quoted
		let _ = '"'.parse_next(input)?;
		let val = take_till(0.., '"').parse_next(input)?;
		let _ = '"'.parse_next(input)?;
		Ok(val.to_string())
	} else if input.starts_with('\'') {
		// single-quoted
		let _ = '\''.parse_next(input)?;
		let val = take_till(0.., '\'').parse_next(input)?;
		let _ = '\''.parse_next(input)?;
		Ok(val.to_string())
	} else {
		// unquoted: terminated by whitespace, >, or /
		let val = take_while(1.., |ch: char| {
			!ch.is_ascii_whitespace()
				&& ch != '>' && ch != '/'
				&& ch != '\''
				&& ch != '"'
		})
		.parse_next(input)?;
		Ok(val.to_string())
	}
}

/// Parse text content between tags. Terminates at `<` or `{` (when expressions enabled).
fn parse_text(
	expressions_enabled: bool,
) -> impl FnMut(&mut &str) -> ModalResult<HtmlToken, ContextError> {
	move |input: &mut &str| {
		let text = if expressions_enabled {
			take_while(1.., |ch: char| ch != '<' && ch != '{')
				.parse_next(input)?
		} else {
			take_while(1.., |ch: char| ch != '<').parse_next(input)?
		};
		Ok(HtmlToken::Text(text.to_string()))
	}
}

/// Parse a `{expression}` with nested brace tracking.
fn parse_expression(input: &mut &str) -> ModalResult<HtmlToken, ContextError> {
	let content = parse_brace_content(input)?;
	Ok(HtmlToken::Expression(content))
}

/// Parse the content between `{` and `}`, handling nested braces.
fn parse_brace_content(input: &mut &str) -> ModalResult<String, ContextError> {
	let _ = '{'.parse_next(input)?;
	let mut depth: u32 = 1;
	let mut content = String::new();

	while depth > 0 {
		if input.is_empty() {
			return Err(ErrMode::Cut(ContextError::new()));
		}
		let ch = any.parse_next(input)?;
		match ch {
			'{' => {
				depth += 1;
				content.push(ch);
			}
			'}' => {
				depth -= 1;
				if depth > 0 {
					content.push(ch);
				}
			}
			_ => content.push(ch),
		}
	}
	Ok(content)
}

/// Parse raw text content inside elements like `<script>` or `<style>`.
/// Content is terminated only by the matching close tag `</name>`.
///
/// When `parse_raw_text_expressions` is enabled, `{{expr}}` sequences
/// are extracted as separate expression tokens.
#[allow(dead_code)]
pub fn parse_raw_text_content(
	tag_name: &str,
	parse_expressions: bool,
) -> impl FnMut(&mut &str) -> ModalResult<Vec<HtmlToken>, ContextError> + '_ {
	move |input: &mut &str| {
		let mut tokens = Vec::new();
		let mut text_buf = String::new();

		while !input.is_empty() {
			// check for closing tag (case-insensitive)
			if starts_with_close_tag(input, tag_name) {
				break;
			}

			// check for double-brace expression
			if parse_expressions && input.starts_with("{{") {
				// flush text buffer
				if !text_buf.is_empty() {
					tokens
						.push(HtmlToken::Text(core::mem::take(&mut text_buf)));
				}
				let _ = "{{".parse_next(input)?;
				let mut expr = String::new();
				// track double-braces: we need to find matching `}}`
				while !input.is_empty() {
					if input.starts_with("}}") {
						let _ = "}}".parse_next(input)?;
						break;
					}
					let ch = any.parse_next(input)?;
					expr.push(ch);
				}
				tokens.push(HtmlToken::Expression(expr));
				continue;
			}

			let ch = any.parse_next(input)?;
			text_buf.push(ch);
		}

		if !text_buf.is_empty() {
			tokens.push(HtmlToken::Text(text_buf));
		}

		Ok(tokens)
	}
}

/// Check if input starts with a close tag for the given element name
/// (case-insensitive comparison).
#[allow(dead_code)]
fn starts_with_close_tag(input: &str, tag_name: &str) -> bool {
	if !input.starts_with("</") {
		return false;
	}
	let rest = &input[2..];
	let rest = rest.trim_start();
	if rest.len() < tag_name.len() {
		return false;
	}
	let candidate = &rest[..tag_name.len()];
	if !candidate.eq_ignore_ascii_case(tag_name) {
		return false;
	}
	// must be followed by whitespace or >
	let after = &rest[tag_name.len()..];
	after.starts_with('>')
		|| after
			.chars()
			.next()
			.map(|ch| ch.is_ascii_whitespace())
			.unwrap_or(false)
		|| after.is_empty()
}


#[cfg(test)]
mod test {
	use super::*;
	use beet_core::prelude::*;

	// -- tag name --

	#[test]
	fn tag_name_simple() {
		let mut input = "div ";
		parse_tag_name(&mut input)
			.unwrap()
			.xpect_eq("div".to_string());
	}

	#[test]
	fn tag_name_custom_element() {
		let mut input = "my-component>";
		parse_tag_name(&mut input)
			.unwrap()
			.xpect_eq("my-component".to_string());
	}

	// -- attribute value --

	#[test]
	fn attr_value_double_quoted() {
		let mut input = "\"hello world\"";
		parse_attribute_value(&mut input)
			.unwrap()
			.xpect_eq("hello world".to_string());
	}

	#[test]
	fn attr_value_single_quoted() {
		let mut input = "'hello'";
		parse_attribute_value(&mut input)
			.unwrap()
			.xpect_eq("hello".to_string());
	}

	#[test]
	fn attr_value_unquoted() {
		let mut input = "hello>";
		parse_attribute_value(&mut input)
			.unwrap()
			.xpect_eq("hello".to_string());
	}

	// -- attributes --

	#[test]
	fn attr_key_value() {
		let mut input = "class=\"foo\"";
		parse_attribute(&mut input)
			.unwrap()
			.xpect_eq(HtmlAttribute::new("class", "foo"));
	}

	#[test]
	fn attr_boolean() {
		let mut input = "disabled>";
		parse_attribute(&mut input)
			.unwrap()
			.xpect_eq(HtmlAttribute::boolean("disabled"));
	}

	#[test]
	fn attr_keyless_expression() {
		let mut input = "{foo}";
		parse_attribute(&mut input)
			.unwrap()
			.xpect_eq(HtmlAttribute::expression("foo"));
	}

	#[test]
	fn attr_keyed_expression() {
		let mut input = "onclick={handler}>";
		parse_attribute(&mut input)
			.unwrap()
			.xpect_eq(HtmlAttribute::keyed_expression("onclick", "handler"));
	}

	// -- comment --

	#[test]
	fn comment_simple() {
		let mut input = "<!-- hello -->";
		parse_comment(&mut input)
			.unwrap()
			.xpect_eq(HtmlToken::Comment(" hello ".to_string()));
	}

	// -- doctype --

	#[test]
	fn doctype_html() {
		let mut input = "<!DOCTYPE html>";
		parse_doctype(&mut input)
			.unwrap()
			.xpect_eq(HtmlToken::Doctype("DOCTYPE html".to_string()));
	}

	// -- close tag --

	#[test]
	fn close_tag_simple() {
		let mut input = "</div>";
		parse_close_tag(&mut input)
			.unwrap()
			.xpect_eq(HtmlToken::CloseTag("div".to_string()));
	}

	#[test]
	fn close_tag_whitespace() {
		let mut input = "</ div >";
		parse_close_tag(&mut input)
			.unwrap()
			.xpect_eq(HtmlToken::CloseTag("div".to_string()));
	}

	// -- open tag --

	#[test]
	fn open_tag_simple() {
		let mut input = "<div>";
		parse_open_tag(&mut input)
			.unwrap()
			.xpect_eq(HtmlToken::OpenTag {
				name: "div".to_string(),
				attributes: vec![],
				self_closing: false,
			});
	}

	#[test]
	fn open_tag_self_closing() {
		let mut input = "<br />";
		parse_open_tag(&mut input)
			.unwrap()
			.xpect_eq(HtmlToken::OpenTag {
				name: "br".to_string(),
				attributes: vec![],
				self_closing: true,
			});
	}

	#[test]
	fn open_tag_with_attributes() {
		let mut input = "<input type=\"text\" disabled />";
		parse_open_tag(&mut input)
			.unwrap()
			.xpect_eq(HtmlToken::OpenTag {
				name: "input".to_string(),
				attributes: vec![
					HtmlAttribute::new("type", "text"),
					HtmlAttribute::boolean("disabled"),
				],
				self_closing: true,
			});
	}

	// -- text --

	#[test]
	fn text_simple() {
		let mut input = "hello world<";
		parse_text(false)(&mut input)
			.unwrap()
			.xpect_eq(HtmlToken::Text("hello world".to_string()));
	}

	#[test]
	fn text_stops_at_brace_when_expressions_enabled() {
		let mut input = "hello {world}";
		parse_text(true)(&mut input)
			.unwrap()
			.xpect_eq(HtmlToken::Text("hello ".to_string()));
	}

	// -- expression --

	#[test]
	fn expression_simple() {
		let mut input = "{foo}";
		parse_expression(&mut input)
			.unwrap()
			.xpect_eq(HtmlToken::Expression("foo".to_string()));
	}

	#[test]
	fn expression_nested_braces() {
		let mut input = "{a {b} c}";
		parse_expression(&mut input)
			.unwrap()
			.xpect_eq(HtmlToken::Expression("a {b} c".to_string()));
	}

	// -- brace content --

	#[test]
	fn brace_content_deeply_nested() {
		let mut input = "{a {b {c}} d}rest";
		parse_brace_content(&mut input)
			.unwrap()
			.xpect_eq("a {b {c}} d".to_string());
		input.xpect_eq("rest");
	}

	// -- raw text --

	#[test]
	fn raw_text_script() {
		let mut input = "let x = 1 < 2;</script>";
		let tokens =
			parse_raw_text_content("script", false)(&mut input).unwrap();
		tokens.xpect_eq(vec![HtmlToken::Text("let x = 1 < 2;".to_string())]);
		input.xpect_eq("</script>");
	}

	#[test]
	fn raw_text_with_expression() {
		let mut input = "prefix {{expr}} suffix</style>";
		let tokens = parse_raw_text_content("style", true)(&mut input).unwrap();
		tokens.xpect_eq(vec![
			HtmlToken::Text("prefix ".to_string()),
			HtmlToken::Expression("expr".to_string()),
			HtmlToken::Text(" suffix".to_string()),
		]);
	}

	// -- document --

	#[test]
	fn document_simple() {
		let config = ParseConfig::default();
		let tokens = parse_document("<div>hello</div>", &config).unwrap();
		tokens.xpect_eq(vec![
			HtmlToken::OpenTag {
				name: "div".to_string(),
				attributes: vec![],
				self_closing: false,
			},
			HtmlToken::Text("hello".to_string()),
			HtmlToken::CloseTag("div".to_string()),
		]);
	}

	#[test]
	fn document_nested() {
		let config = ParseConfig::default();
		let tokens =
			parse_document("<div><span>hi</span></div>", &config).unwrap();
		tokens.xpect_eq(vec![
			HtmlToken::OpenTag {
				name: "div".to_string(),
				attributes: vec![],
				self_closing: false,
			},
			HtmlToken::OpenTag {
				name: "span".to_string(),
				attributes: vec![],
				self_closing: false,
			},
			HtmlToken::Text("hi".to_string()),
			HtmlToken::CloseTag("span".to_string()),
			HtmlToken::CloseTag("div".to_string()),
		]);
	}

	#[test]
	fn document_with_expressions() {
		let config = ParseConfig {
			parse_expressions: true,
			..Default::default()
		};
		let tokens = parse_document("<p>hello {name}</p>", &config).unwrap();
		tokens.xpect_eq(vec![
			HtmlToken::OpenTag {
				name: "p".to_string(),
				attributes: vec![],
				self_closing: false,
			},
			HtmlToken::Text("hello ".to_string()),
			HtmlToken::Expression("name".to_string()),
			HtmlToken::CloseTag("p".to_string()),
		]);
	}

	#[test]
	fn document_comment_and_doctype() {
		let config = ParseConfig::default();
		let tokens =
			parse_document("<!DOCTYPE html><!-- hi --><br />", &config)
				.unwrap();
		tokens.xpect_eq(vec![
			HtmlToken::Doctype("DOCTYPE html".to_string()),
			HtmlToken::Comment(" hi ".to_string()),
			HtmlToken::OpenTag {
				name: "br".to_string(),
				attributes: vec![],
				self_closing: true,
			},
		]);
	}

	#[test]
	fn document_mixed_content() {
		let config = ParseConfig::default();
		let tokens =
			parse_document("<p>hello <em>world</em></p>", &config).unwrap();
		tokens.xpect_eq(vec![
			HtmlToken::OpenTag {
				name: "p".to_string(),
				attributes: vec![],
				self_closing: false,
			},
			HtmlToken::Text("hello ".to_string()),
			HtmlToken::OpenTag {
				name: "em".to_string(),
				attributes: vec![],
				self_closing: false,
			},
			HtmlToken::Text("world".to_string()),
			HtmlToken::CloseTag("em".to_string()),
			HtmlToken::CloseTag("p".to_string()),
		]);
	}

	#[test]
	fn document_whitespace_preserved() {
		let config = ParseConfig::default();
		let tokens = parse_document("<div>  hello  </div>", &config).unwrap();
		tokens.xpect_eq(vec![
			HtmlToken::OpenTag {
				name: "div".to_string(),
				attributes: vec![],
				self_closing: false,
			},
			HtmlToken::Text("  hello  ".to_string()),
			HtmlToken::CloseTag("div".to_string()),
		]);
	}

	#[test]
	fn document_multiple_attributes() {
		let config = ParseConfig::default();
		let tokens =
			parse_document("<div class=\"foo\" id='bar' data-x=baz>", &config)
				.unwrap();
		tokens.xpect_eq(vec![HtmlToken::OpenTag {
			name: "div".to_string(),
			attributes: vec![
				HtmlAttribute::new("class", "foo"),
				HtmlAttribute::new("id", "bar"),
				HtmlAttribute::new("data-x", "baz"),
			],
			self_closing: false,
		}]);
	}

	#[test]
	fn starts_with_close_tag_check() {
		starts_with_close_tag("</script>", "script").xpect_true();
		starts_with_close_tag("</SCRIPT>", "script").xpect_true();
		starts_with_close_tag("</ script>", "script").xpect_true();
		starts_with_close_tag("</div>", "script").xpect_false();
		starts_with_close_tag("not a tag", "script").xpect_false();
	}
}
