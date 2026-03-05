//! Winnow parser combinators for HTML tokenization.
//!
//! All combinators operate on `&str` input and produce [`HtmlToken`] or
//! sub-components thereof. The parsers are designed to be composed by
//! the top-level [`parse_document`] entry point.
//!
//! All returned string data borrows directly from the input for zero-copy
//! parsing. Owned strings are only created when building ECS components.

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

/// Parse a complete HTML document into a sequence of borrowed tokens.
///
/// When an opening tag for a raw text element (ie `<script>`, `<style>`) is
/// encountered, the parser switches to raw text mode until the matching
/// close tag, correctly handling `<` characters and optional `{{expr}}`
/// expressions inside the raw content.
pub fn parse_document<'a>(
	input: &'a str,
	config: &ParseConfig,
) -> Result<Vec<HtmlToken<'a>>, String> {
	let mut remaining = input;
	let mut tokens = Vec::new();

	while !remaining.is_empty() {
		match parse_node(config).parse_next(&mut remaining) {
			Ok(token) => {
				// after an open tag for a raw text element, switch to raw
				// text mode for its content
				let raw_tag_name = if let HtmlToken::OpenTag {
					name,
					self_closing: false,
					..
				} = &token
				{
					if config.is_raw_text_element(name) {
						Some(*name)
					} else {
						None
					}
				} else {
					None
				};

				tokens.push(token);

				if let Some(tag_name) = raw_tag_name {
					// parse raw text content until the matching close tag
					match parse_raw_text_content(
						tag_name,
						config.parse_raw_text_expressions,
					)(&mut remaining)
					{
						Ok(raw_tokens) => tokens.extend(raw_tokens),
						Err(err) => {
							return Err(format!(
								"parse error in raw text element <{tag_name}> at byte {}: {err}",
								input.len() - remaining.len()
							));
						}
					}
				}
			}
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
fn parse_node<'input, 'config>(
	config: &'config ParseConfig,
) -> impl Parser<&'input str, HtmlToken<'input>, ErrMode<ContextError>> + 'config
where
	'input: 'config,
{
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
fn parse_comment<'a>(
	input: &mut &'a str,
) -> ModalResult<HtmlToken<'a>, ContextError> {
	let _ = "<!--".parse_next(input)?;
	let content = take_until(0.., "-->").parse_next(input)?;
	let _ = "-->".parse_next(input)?;
	Ok(HtmlToken::Comment(content))
}

/// Parse a doctype declaration: `<!DOCTYPE ...>`.
fn parse_doctype<'a>(
	input: &mut &'a str,
) -> ModalResult<HtmlToken<'a>, ContextError> {
	let _ = "<!".parse_next(input)?;
	let content = take_till(1.., '>').parse_next(input)?;
	let _ = '>'.parse_next(input)?;
	Ok(HtmlToken::Doctype(content))
}

/// Parse a closing tag: `</name>`.
fn parse_close_tag<'a>(
	input: &mut &'a str,
) -> ModalResult<HtmlToken<'a>, ContextError> {
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
///
/// Captures the full source text from `<` to `>` (inclusive) in the
/// `source` field for [`FileSpan`] tracking.
fn parse_open_tag<'a>(
	input: &mut &'a str,
) -> ModalResult<HtmlToken<'a>, ContextError> {
	// capture start of input before consuming `<`
	let before = *input;
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
			let source_len = before.len() - input.len();
			return Ok(HtmlToken::OpenTag {
				name,
				attributes,
				self_closing: true,
				source: &before[..source_len],
			});
		}
		if input.starts_with('>') {
			let _ = '>'.parse_next(input)?;
			let source_len = before.len() - input.len();
			return Ok(HtmlToken::OpenTag {
				name,
				attributes,
				self_closing: false,
				source: &before[..source_len],
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
/// alphanumeric, hyphens, underscores, dots, or colons (for SVG/XML namespaces).
fn parse_tag_name<'a>(
	input: &mut &'a str,
) -> ModalResult<&'a str, ContextError> {
	// capture the start of the remaining input
	let before = *input;
	let _first = winnow::token::one_of(|ch: char| ch.is_ascii_alphabetic())
		.parse_next(input)?;
	let _rest =
		take_while(0.., |ch: char| {
			ch.is_ascii_alphanumeric()
				|| ch == '-' || ch == '_'
				|| ch == '.' || ch == ':'
		})
		.parse_next(input)?;
	// the consumed portion is the slice from before up to current position
	let consumed_len = before.len() - input.len();
	Ok(&before[..consumed_len])
}

/// Parse a single attribute from inside an opening tag.
///
/// Supports:
/// - `key="value"` or `key='value'`
/// - `key=unquoted`
/// - `key` (boolean, no value)
/// - `{expr}` (keyless expression)
/// - `key={expr}` (keyed expression)
fn parse_attribute<'a>(
	input: &mut &'a str,
) -> ModalResult<HtmlAttribute<'a>, ContextError> {
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

/// Parse an attribute name: alphanumeric, hyphens, underscores, colons, periods, `@`.
fn parse_attribute_name<'a>(
	input: &mut &'a str,
) -> ModalResult<&'a str, ContextError> {
	let name =
		take_while(1.., |ch: char| {
			ch.is_ascii_alphanumeric()
				|| ch == '-' || ch == '_'
				|| ch == ':' || ch == '.'
				|| ch == '@'
		})
		.parse_next(input)?;
	Ok(name)
}

/// Parse an attribute value: double-quoted, single-quoted, or unquoted.
fn parse_attribute_value<'a>(
	input: &mut &'a str,
) -> ModalResult<&'a str, ContextError> {
	if input.starts_with('"') {
		// double-quoted
		let _ = '"'.parse_next(input)?;
		let val = take_till(0.., '"').parse_next(input)?;
		let _ = '"'.parse_next(input)?;
		Ok(val)
	} else if input.starts_with('\'') {
		// single-quoted
		let _ = '\''.parse_next(input)?;
		let val = take_till(0.., '\'').parse_next(input)?;
		let _ = '\''.parse_next(input)?;
		Ok(val)
	} else {
		// unquoted: terminated by whitespace, >, or /
		let val = take_while(1.., |ch: char| {
			!ch.is_ascii_whitespace()
				&& ch != '>' && ch != '/'
				&& ch != '\''
				&& ch != '"'
		})
		.parse_next(input)?;
		Ok(val)
	}
}

/// Parse text content between tags. Terminates at `<` or `{` (when expressions enabled).
fn parse_text<'a>(
	expressions_enabled: bool,
) -> impl FnMut(&mut &'a str) -> ModalResult<HtmlToken<'a>, ContextError> {
	move |input: &mut &'a str| {
		let text = if expressions_enabled {
			take_while(1.., |ch: char| ch != '<' && ch != '{')
				.parse_next(input)?
		} else {
			take_while(1.., |ch: char| ch != '<').parse_next(input)?
		};
		Ok(HtmlToken::Text(text))
	}
}

/// Parse a `{expression}` with nested brace tracking, returning a borrowed slice.
fn parse_expression<'a>(
	input: &mut &'a str,
) -> ModalResult<HtmlToken<'a>, ContextError> {
	let content = parse_brace_content(input)?;
	Ok(HtmlToken::Expression(content))
}

/// Parse the content between `{` and `}`, handling nested braces.
/// Returns a borrowed slice of the content (excluding the outer braces).
fn parse_brace_content<'a>(
	input: &mut &'a str,
) -> ModalResult<&'a str, ContextError> {
	let _ = '{'.parse_next(input)?;
	// after consuming `{`, the remaining input starts at the content
	let content_start = *input;
	let mut depth: u32 = 1;

	while depth > 0 {
		if input.is_empty() {
			return Err(ErrMode::Cut(ContextError::new()));
		}
		let ch = any.parse_next(input)?;
		match ch {
			'{' => {
				depth += 1;
			}
			'}' => {
				depth -= 1;
			}
			_ => {}
		}
	}
	// the content is everything from content_start up to (but not including)
	// the final `}` which was just consumed
	let content_len = content_start.len() - input.len() - 1; // -1 for closing `}`
	Ok(&content_start[..content_len])
}

/// Parse raw text content inside elements like `<script>` or `<style>`.
/// Content is terminated only by the matching close tag `</name>`.
///
/// When `parse_raw_text_expressions` is enabled, `{{expr}}` sequences
/// are extracted as separate expression tokens.
pub fn parse_raw_text_content<'a, 'tag>(
	tag_name: &'tag str,
	parse_expressions: bool,
) -> impl FnMut(&mut &'a str) -> ModalResult<Vec<HtmlToken<'a>>, ContextError> + 'tag
where
	'a: 'tag,
{
	move |input: &mut &'a str| {
		let mut tokens = Vec::new();
		// track start of current text run
		let mut text_start = *input;

		while !input.is_empty() {
			// check for closing tag (case-insensitive)
			if starts_with_close_tag(input, tag_name) {
				// flush text buffer
				let text_len = text_start.len() - input.len();
				if text_len > 0 {
					tokens.push(HtmlToken::Text(&text_start[..text_len]));
				}
				break;
			}

			// check for double-brace expression
			if parse_expressions && input.starts_with("{{") {
				// flush text buffer up to this point
				let text_len = text_start.len() - input.len();
				if text_len > 0 {
					tokens.push(HtmlToken::Text(&text_start[..text_len]));
				}
				let _ = "{{".parse_next(input)?;
				// capture expression start
				let expr_start = *input;
				while !input.is_empty() {
					if input.starts_with("}}") {
						let expr_len = expr_start.len() - input.len();
						let expr = &expr_start[..expr_len];
						let _ = "}}".parse_next(input)?;
						tokens.push(HtmlToken::Expression(expr));
						break;
					}
					let _ = any.parse_next(input)?;
				}
				// reset text start after expression
				text_start = *input;
				continue;
			}

			let _ = any.parse_next(input)?;
		}

		// flush remaining text if we hit EOF without close tag
		if !starts_with_close_tag_or_empty(input, tag_name) {
			let text_len = text_start.len() - input.len();
			if text_len > 0 {
				tokens.push(HtmlToken::Text(&text_start[..text_len]));
			}
		}

		Ok(tokens)
	}
}

/// Check if input starts with a close tag for the given element name
/// (case-insensitive comparison).
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

/// Returns true if input is empty or starts with the given close tag.
fn starts_with_close_tag_or_empty(input: &str, tag_name: &str) -> bool {
	input.is_empty() || starts_with_close_tag(input, tag_name)
}


#[cfg(test)]
mod test {
	use super::*;
	use beet_core::prelude::*;

	// -- tag name --

	#[test]
	fn tag_name_simple() {
		let mut input = "div ";
		parse_tag_name(&mut input).unwrap().xpect_eq("div");
	}

	#[test]
	fn tag_name_custom_element() {
		let mut input = "my-component>";
		parse_tag_name(&mut input).unwrap().xpect_eq("my-component");
	}

	#[test]
	fn tag_name_svg_namespace() {
		let mut input = "svg:circle>";
		parse_tag_name(&mut input).unwrap().xpect_eq("svg:circle");
	}

	// -- attribute value --

	#[test]
	fn attr_value_double_quoted() {
		let mut input = "\"hello world\"";
		parse_attribute_value(&mut input)
			.unwrap()
			.xpect_eq("hello world");
	}

	#[test]
	fn attr_value_single_quoted() {
		let mut input = "'hello'";
		parse_attribute_value(&mut input).unwrap().xpect_eq("hello");
	}

	#[test]
	fn attr_value_unquoted() {
		let mut input = "hello>";
		parse_attribute_value(&mut input).unwrap().xpect_eq("hello");
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
			.xpect_eq(HtmlToken::Comment(" hello "));
	}

	// -- doctype --

	#[test]
	fn doctype_html() {
		let mut input = "<!DOCTYPE html>";
		parse_doctype(&mut input)
			.unwrap()
			.xpect_eq(HtmlToken::Doctype("DOCTYPE html"));
	}

	// -- close tag --

	#[test]
	fn close_tag_simple() {
		let mut input = "</div>";
		parse_close_tag(&mut input)
			.unwrap()
			.xpect_eq(HtmlToken::CloseTag("div"));
	}

	#[test]
	fn close_tag_whitespace() {
		let mut input = "</ div >";
		parse_close_tag(&mut input)
			.unwrap()
			.xpect_eq(HtmlToken::CloseTag("div"));
	}

	// -- open tag --

	#[test]
	fn open_tag_simple() {
		let mut input = "<div>";
		parse_open_tag(&mut input)
			.unwrap()
			.xpect_eq(HtmlToken::OpenTag {
				name: "div",
				attributes: vec![],
				self_closing: false,
				source: "<div>",
			});
	}

	#[test]
	fn open_tag_self_closing() {
		let mut input = "<br />";
		parse_open_tag(&mut input)
			.unwrap()
			.xpect_eq(HtmlToken::OpenTag {
				name: "br",
				attributes: vec![],
				self_closing: true,
				source: "<br />",
			});
	}

	#[test]
	fn open_tag_with_attributes() {
		let mut input = "<input type=\"text\" disabled />";
		parse_open_tag(&mut input)
			.unwrap()
			.xpect_eq(HtmlToken::OpenTag {
				name: "input",
				attributes: vec![
					HtmlAttribute::new("type", "text"),
					HtmlAttribute::boolean("disabled"),
				],
				self_closing: true,
				source: "<input type=\"text\" disabled />",
			});
	}

	// -- text --

	#[test]
	fn text_simple() {
		let mut input = "hello world<";
		parse_text(false)(&mut input)
			.unwrap()
			.xpect_eq(HtmlToken::Text("hello world"));
	}

	#[test]
	fn text_stops_at_brace_when_expressions_enabled() {
		let mut input = "hello {world}";
		parse_text(true)(&mut input)
			.unwrap()
			.xpect_eq(HtmlToken::Text("hello "));
	}

	// -- expression --

	#[test]
	fn expression_simple() {
		let mut input = "{foo}";
		parse_expression(&mut input)
			.unwrap()
			.xpect_eq(HtmlToken::Expression("foo"));
	}

	#[test]
	fn expression_nested_braces() {
		let mut input = "{a {b} c}";
		parse_expression(&mut input)
			.unwrap()
			.xpect_eq(HtmlToken::Expression("a {b} c"));
	}

	// -- brace content --

	#[test]
	fn brace_content_deeply_nested() {
		let mut input = "{a {b {c}} d}rest";
		parse_brace_content(&mut input)
			.unwrap()
			.xpect_eq("a {b {c}} d");
		input.xpect_eq("rest");
	}

	// -- raw text --

	#[test]
	fn raw_text_script() {
		let mut input = "let x = 1 < 2;</script>";
		let tokens =
			parse_raw_text_content("script", false)(&mut input).unwrap();
		tokens.xpect_eq(vec![HtmlToken::Text("let x = 1 < 2;")]);
		input.xpect_eq("</script>");
	}

	#[test]
	fn raw_text_with_expression() {
		let mut input = "prefix {{expr}} suffix</style>";
		let tokens = parse_raw_text_content("style", true)(&mut input).unwrap();
		tokens.xpect_eq(vec![
			HtmlToken::Text("prefix "),
			HtmlToken::Expression("expr"),
			HtmlToken::Text(" suffix"),
		]);
	}

	// -- raw text in document --

	#[test]
	fn document_script_raw_text() {
		let config = ParseConfig::default();
		let tokens =
			parse_document("<script>let x = 1 < 2;</script>", &config).unwrap();
		tokens.xpect_eq(vec![
			HtmlToken::OpenTag {
				name: "script",
				attributes: vec![],
				self_closing: false,
				source: "<script>",
			},
			HtmlToken::Text("let x = 1 < 2;"),
			HtmlToken::CloseTag("script"),
		]);
	}

	#[test]
	fn document_style_raw_text() {
		let config = ParseConfig::default();
		let tokens =
			parse_document("<style>body { color: red; }</style>", &config)
				.unwrap();
		tokens.xpect_eq(vec![
			HtmlToken::OpenTag {
				name: "style",
				attributes: vec![],
				self_closing: false,
				source: "<style>",
			},
			HtmlToken::Text("body { color: red; }"),
			HtmlToken::CloseTag("style"),
		]);
	}

	#[test]
	fn document_script_with_expressions() {
		let config = ParseConfig {
			parse_raw_text_expressions: true,
			..Default::default()
		};
		let tokens =
			parse_document("<script>prefix {{name}} suffix</script>", &config)
				.unwrap();
		tokens.xpect_eq(vec![
			HtmlToken::OpenTag {
				name: "script",
				attributes: vec![],
				self_closing: false,
				source: "<script>",
			},
			HtmlToken::Text("prefix "),
			HtmlToken::Expression("name"),
			HtmlToken::Text(" suffix"),
			HtmlToken::CloseTag("script"),
		]);
	}

	#[test]
	fn document_self_closing_script_no_raw() {
		// self-closing script should NOT trigger raw text mode
		let config = ParseConfig::default();
		let tokens =
			parse_document("<script /><div>hello</div>", &config).unwrap();
		tokens.len().xpect_eq(4); // script(self-closing), open-div, text, close-div
	}

	// -- document --

	#[test]
	fn document_simple() {
		let config = ParseConfig::default();
		let tokens = parse_document("<div>hello</div>", &config).unwrap();
		tokens.xpect_eq(vec![
			HtmlToken::OpenTag {
				name: "div",
				attributes: vec![],
				self_closing: false,
				source: "<div>",
			},
			HtmlToken::Text("hello"),
			HtmlToken::CloseTag("div"),
		]);
	}

	#[test]
	fn document_nested() {
		let config = ParseConfig::default();
		let tokens =
			parse_document("<div><span>hi</span></div>", &config).unwrap();
		tokens.xpect_eq(vec![
			HtmlToken::OpenTag {
				name: "div",
				attributes: vec![],
				self_closing: false,
				source: "<div>",
			},
			HtmlToken::OpenTag {
				name: "span",
				attributes: vec![],
				self_closing: false,
				source: "<span>",
			},
			HtmlToken::Text("hi"),
			HtmlToken::CloseTag("span"),
			HtmlToken::CloseTag("div"),
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
				name: "p",
				attributes: vec![],
				self_closing: false,
				source: "<p>",
			},
			HtmlToken::Text("hello "),
			HtmlToken::Expression("name"),
			HtmlToken::CloseTag("p"),
		]);
	}

	#[test]
	fn document_comment_and_doctype() {
		let config = ParseConfig::default();
		let tokens =
			parse_document("<!DOCTYPE html><!-- hi --><br />", &config)
				.unwrap();
		tokens.xpect_eq(vec![
			HtmlToken::Doctype("DOCTYPE html"),
			HtmlToken::Comment(" hi "),
			HtmlToken::OpenTag {
				name: "br",
				attributes: vec![],
				self_closing: true,
				source: "<br />",
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
				name: "p",
				attributes: vec![],
				self_closing: false,
				source: "<p>",
			},
			HtmlToken::Text("hello "),
			HtmlToken::OpenTag {
				name: "em",
				attributes: vec![],
				self_closing: false,
				source: "<em>",
			},
			HtmlToken::Text("world"),
			HtmlToken::CloseTag("em"),
			HtmlToken::CloseTag("p"),
		]);
	}

	#[test]
	fn document_whitespace_preserved() {
		let config = ParseConfig::default();
		let tokens = parse_document("<div>  hello  </div>", &config).unwrap();
		tokens.xpect_eq(vec![
			HtmlToken::OpenTag {
				name: "div",
				attributes: vec![],
				self_closing: false,
				source: "<div>",
			},
			HtmlToken::Text("  hello  "),
			HtmlToken::CloseTag("div"),
		]);
	}

	#[test]
	fn document_multiple_attributes() {
		let config = ParseConfig::default();
		let tokens =
			parse_document("<div class=\"foo\" id='bar' data-x=baz>", &config)
				.unwrap();
		tokens.xpect_eq(vec![HtmlToken::OpenTag {
			name: "div",
			attributes: vec![
				HtmlAttribute::new("class", "foo"),
				HtmlAttribute::new("id", "bar"),
				HtmlAttribute::new("data-x", "baz"),
			],
			self_closing: false,
			source: "<div class=\"foo\" id='bar' data-x=baz>",
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

	// -- SVG --

	#[test]
	fn svg_basic_elements() {
		let config = ParseConfig::default();
		let input = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 100 100\"><circle cx=\"50\" cy=\"50\" r=\"40\" /></svg>";
		let tokens = parse_document(input, &config).unwrap();
		tokens.len().xpect_eq(3);
		match &tokens[0] {
			HtmlToken::OpenTag {
				name, attributes, ..
			} => {
				name.xpect_eq("svg");
				attributes.len().xpect_eq(2);
			}
			other => panic!("expected OpenTag, got {other:?}"),
		}
		match &tokens[1] {
			HtmlToken::OpenTag {
				name,
				self_closing,
				attributes,
				..
			} => {
				name.xpect_eq("circle");
				self_closing.xpect_true();
				attributes.len().xpect_eq(3);
			}
			other => panic!("expected OpenTag, got {other:?}"),
		}
	}

	#[test]
	fn svg_path_data() {
		let config = ParseConfig::default();
		let tokens = parse_document(
			"<path d=\"M10 10 H 90 V 90 H 10 Z\" fill=\"none\" stroke=\"black\" />",
			&config,
		)
		.unwrap();
		tokens.len().xpect_eq(1);
		match &tokens[0] {
			HtmlToken::OpenTag {
				name,
				attributes,
				self_closing,
				..
			} => {
				name.xpect_eq("path");
				self_closing.xpect_true();
				attributes.len().xpect_eq(3);
				attributes[0]
					.value
					.unwrap()
					.xpect_eq("M10 10 H 90 V 90 H 10 Z");
			}
			other => panic!("expected OpenTag, got {other:?}"),
		}
	}

	#[test]
	fn svg_namespace_attribute() {
		let config = ParseConfig::default();
		let tokens =
			parse_document("<use xlink:href=\"#icon\" />", &config).unwrap();
		match &tokens[0] {
			HtmlToken::OpenTag { attributes, .. } => {
				attributes[0].key.xpect_eq("xlink:href");
				attributes[0].value.unwrap().xpect_eq("#icon");
			}
			other => panic!("expected OpenTag, got {other:?}"),
		}
	}
}
