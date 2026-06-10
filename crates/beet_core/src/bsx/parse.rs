//! The hand-written recursive-descent markup parser.
//!
//! One grammar: BSX with its extra surface enabled, HTML with it disabled.
//! [`BsxParseConfig::bsx`] gates the value grammar, `{..}` blocks, and
//! `bx:` directives, so the HTML-only mode is a real, tested configuration. The
//! parser produces a [`BsxNode`] tree; resolution into the world is
//! [`super::resolve`].

use super::ast::*;
use super::cursor::Cursor;
use super::value::*;
use crate::prelude::*;

/// Configuration toggling the BSX-only grammar surface.
#[derive(Debug, Clone)]
pub struct BsxParseConfig {
	/// When `true`, the value grammar (`{..}` literals/references, bare spreads)
	/// and `bx:` directives are parsed. When `false`, the parser accepts exactly
	/// HTML: lowercase tags, string attributes, text, comments.
	pub bsx: bool,
}

impl Default for BsxParseConfig {
	fn default() -> Self { Self { bsx: true } }
}

impl BsxParseConfig {
	/// The full BSX grammar.
	pub fn bsx() -> Self { Self { bsx: true } }
	/// HTML-only: the BSX surface disabled.
	pub fn html() -> Self { Self { bsx: false } }
}

/// HTML void elements that never have a closing tag.
const VOID_ELEMENTS: &[&str] = &[
	"area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta",
	"param", "source", "track", "wbr",
];

/// Elements whose content is raw text up to their closing tag.
const RAW_TEXT_ELEMENTS: &[&str] = &["script", "style"];

/// Parse a full document into a flat list of root nodes.
pub fn parse_document(
	source: &str,
	config: &BsxParseConfig,
) -> Result<Vec<BsxNode>> {
	let mut cursor = Cursor::new(source);
	let nodes = parse_nodes(&mut cursor, config, None)?;
	Ok(nodes)
}

/// One lenient, fragment-safe markup token produced by [`parse_fragment`].
///
/// Unlike [`BsxNode`], a token is *flat*: an open tag and its close are separate
/// tokens, so a fragment need not balance. This is what lets a stack-based caller
/// (the markdown interleaving builder) feed BSX one `pulldown-cmark` HTML
/// fragment at a time, where the open tag, the markdown between it, and the close
/// tag arrive as separate events.
///
/// Verbatim spans (tag name, text, comment, doctype, raw `{expr}`) borrow from
/// the input for zero-copy span tracking; only the resolved attribute list is
/// owned, since the value grammar produces owned literals.
#[derive(Debug, Clone, PartialEq)]
pub enum BsxFragmentToken<'a> {
	/// An opening tag with its tag name, resolved attributes, and whether it is
	/// self-closing (`<br/>`) or a void element.
	Open {
		/// The raw tag text, ie `div`, `MyTemplate`, `path::to::X`.
		tag: &'a str,
		/// The resolved attributes, sharing the [`AttrValue`] grammar with
		/// [`BsxElement`] so `bx:` directives and spreads resolve natively.
		attributes: Vec<BsxAttribute>,
		/// Whether the tag self-closes, so the caller emits no matching close.
		self_closing: bool,
	},
	/// A closing tag `</name>`.
	Close {
		/// The closed tag name.
		tag: &'a str,
	},
	/// Verbatim text between tags, entities undecoded (the caller decodes).
	Text(&'a str),
	/// The inner content of an `<!-- .. -->` comment.
	Comment(&'a str),
	/// The value of a `<!DOCTYPE ..>`, the `DOCTYPE` keyword stripped.
	Doctype(&'a str),
	/// The raw inner text of an `{expr}` block, when expressions are enabled.
	Expr(&'a str),
}

/// Configuration for [`parse_fragment`], mirroring the markup quirks an embedded
/// HTML fragment needs: `{expr}` splitting, `<script>`/`<style>` raw text, and
/// `{{expr}}` inside that raw text.
#[derive(Debug, Clone)]
pub struct BsxFragmentConfig {
	/// Whether `{expr}` blocks are split out of text and attribute position.
	pub expressions: bool,
	/// Whether `{{expr}}` blocks inside raw-text elements are split out.
	pub raw_text_expressions: bool,
	/// Element names whose content is raw text, ie `script`, `style`.
	pub raw_text_elements: Vec<SmolStr>,
	/// Element names whose content is raw character data, ie `textarea`, `title`.
	pub raw_character_data_elements: Vec<SmolStr>,
}

impl Default for BsxFragmentConfig {
	fn default() -> Self {
		Self {
			expressions: false,
			raw_text_expressions: false,
			raw_text_elements: vec!["script".into(), "style".into()],
			raw_character_data_elements: vec![
				"textarea".into(),
				"title".into(),
			],
		}
	}
}

impl BsxFragmentConfig {
	/// Whether `name` is a raw-text or raw-character-data element.
	fn is_raw_text_element(&self, name: &str) -> bool {
		let matches = |list: &[SmolStr]| {
			list.iter().any(|el| el.eq_ignore_ascii_case(name))
		};
		matches(&self.raw_text_elements)
			|| matches(&self.raw_character_data_elements)
	}
}

/// Tokenize a markup fragment leniently into a flat [`BsxFragmentToken`] stream.
///
/// This is the fragment-level entry the markdown builder drives instead of its
/// own HTML tokenizer: it reuses the BSX cursor, attribute parser, and value
/// grammar, so embedded markup resolves through the one parser. It never
/// requires the fragment to balance, a lone `<span>` or a lone `</span>` each
/// produce a single token, because `pulldown-cmark` splits a tag and its close
/// across separate events. It is no_std-clean (allocates only the attribute
/// list, borrows every verbatim span from `source`).
pub fn parse_fragment<'a>(
	source: &'a str,
	config: &BsxFragmentConfig,
) -> Result<Vec<BsxFragmentToken<'a>>> {
	let mut cursor = Cursor::new(source);
	let mut tokens = Vec::new();
	while !cursor.is_eof() {
		let token = parse_fragment_token(&mut cursor, config)?;
		let Some(token) = token else { continue };
		// after an open tag for a raw-text element, take its content verbatim up
		// to the matching close tag, so `<` inside `<script>` is not misparsed.
		if let BsxFragmentToken::Open {
			tag,
			self_closing: false,
			..
		} = &token
		{
			if config.is_raw_text_element(tag) {
				let tag = *tag;
				tokens.push(token);
				parse_raw_text(&mut cursor, tag, config, &mut tokens);
				continue;
			}
		}
		tokens.push(token);
	}
	Ok(tokens)
}

/// Parse one fragment token, or `None` for a stray char that was skipped.
fn parse_fragment_token<'a>(
	cursor: &mut Cursor<'a>,
	config: &BsxFragmentConfig,
) -> Result<Option<BsxFragmentToken<'a>>> {
	if cursor.starts_with("<!--") {
		cursor.eat("<!--");
		let content = cursor.take_until("-->");
		cursor.eat("-->");
		return Ok(Some(BsxFragmentToken::Comment(content)));
	}
	if cursor.starts_with("<!") {
		cursor.eat("<!");
		let raw = cursor.take_until(">");
		cursor.eat(">");
		let value = raw
			.trim()
			.strip_prefix("DOCTYPE")
			.or_else(|| raw.trim().strip_prefix("doctype"))
			.map(str::trim)
			.unwrap_or(raw);
		return Ok(Some(BsxFragmentToken::Doctype(value)));
	}
	if cursor.starts_with("</") {
		cursor.eat("</");
		cursor.skip_ws();
		let tag = cursor.take_while(is_tag_char);
		cursor.skip_ws();
		cursor.eat(">");
		return Ok(Some(BsxFragmentToken::Close { tag }));
	}
	if cursor.starts_with("<") {
		return parse_fragment_open(cursor).map(Some);
	}
	if config.expressions && cursor.peek() == Some('{') {
		let start = cursor.offset() + 1;
		take_braced(cursor)?;
		// the inner span is the braced body, sans the closing `}`.
		let inner = cursor.slice(start, cursor.offset() - 1);
		return Ok(Some(BsxFragmentToken::Expr(inner)));
	}
	// verbatim text up to the next `<` (and `{` when expressions are enabled).
	let start = cursor.offset();
	let text = cursor.take_while(|ch| {
		ch != '<' && !(config.expressions && ch == '{')
	});
	if text.is_empty() {
		// a stray char (eg a `<` not starting a valid tag): consume it as text so
		// the loop advances, matching lenient HTML.
		if !cursor.is_eof() {
			cursor.bump();
		}
		let consumed = cursor.slice(start, cursor.offset());
		return Ok((!consumed.is_empty())
			.then_some(BsxFragmentToken::Text(consumed)));
	}
	Ok(Some(BsxFragmentToken::Text(text)))
}

/// Parse an opening tag into a fragment token.
fn parse_fragment_open<'a>(
	cursor: &mut Cursor<'a>,
) -> Result<BsxFragmentToken<'a>> {
	cursor.eat("<");
	cursor.skip_ws();
	let tag = cursor.take_while(is_tag_char);
	if tag.is_empty() {
		bevybail!("expected a tag name after `<`");
	}
	let attributes = parse_fragment_attributes(cursor)?;
	cursor.skip_ws();
	// a `<br/>` self-closes; otherwise consume `>` and let the caller treat void
	// elements (`<br>`) as self-closing too, matching the document parser.
	let self_closing = if cursor.eat("/>") {
		true
	} else {
		cursor.eat(">");
		VOID_ELEMENTS.contains(&tag)
	};
	Ok(BsxFragmentToken::Open {
		tag,
		attributes,
		self_closing,
	})
}

/// Parse the attributes of a fragment opening tag.
///
/// Embedded markup is HTML with BSX extensions: an unquoted `key=value` is a
/// plain string (HTML), so a URL like `src=https://x//` is not run through the
/// value grammar; only braced `key={..}` / bare `{..}` use the value grammar so
/// `bx:` directives, spreads, and references still resolve.
fn parse_fragment_attributes(
	cursor: &mut Cursor,
) -> Result<Vec<BsxAttribute>> {
	let mut attributes = Vec::new();
	loop {
		cursor.skip_ws();
		match cursor.peek() {
			None | Some('>') => break,
			Some('/') if cursor.starts_with("/>") => break,
			// a bare-position spread `<el {..}>`.
			Some('{') => {
				let inner = take_braced(cursor)?;
				let spread = parse_spread(&mut Cursor::new(&inner))?;
				attributes.push(BsxAttribute {
					key: String::new(),
					value: AttrValue::Spread(spread),
				});
			}
			_ => attributes.push(parse_fragment_attribute(cursor)?),
		}
	}
	Ok(attributes)
}

/// Parse one fragment attribute: `key`, `key="v"`, `key=unquoted`, `key={..}`.
fn parse_fragment_attribute(cursor: &mut Cursor) -> Result<BsxAttribute> {
	let key = cursor.take_while(is_attr_key_char).to_string();
	if key.is_empty() {
		bevybail!("expected an attribute name");
	}
	cursor.skip_ws();
	if !cursor.eat("=") {
		return Ok(BsxAttribute {
			key,
			value: AttrValue::Flag,
		});
	}
	cursor.skip_ws();
	let value = match cursor.peek() {
		Some('"') | Some('\'') => AttrValue::Str(parse_fragment_string(cursor)),
		Some('{') => {
			let inner = take_braced(cursor)?;
			AttrValue::Expr(parse_value_expr(&mut Cursor::new(&inner))?)
		}
		// unquoted HTML value: a plain string up to whitespace, `>` or `/>`. Per
		// the HTML spec `/` does not terminate it, so URLs survive.
		_ => AttrValue::Str(parse_unquoted_value(cursor)),
	};
	Ok(BsxAttribute { key, value })
}

/// Read a quoted attribute string, returning its inner text verbatim.
fn parse_fragment_string(cursor: &mut Cursor) -> String {
	let quote = cursor.bump().unwrap();
	let mut out = String::new();
	while let Some(ch) = cursor.bump() {
		if ch == quote {
			break;
		}
		out.push(ch);
	}
	out
}

/// Read an unquoted HTML attribute value, terminated by whitespace, `>`, `=`,
/// quotes, or a backtick (but not `/`, so a trailing-slash URL survives).
fn parse_unquoted_value(cursor: &mut Cursor) -> String {
	cursor
		.take_while(|ch| {
			!ch.is_whitespace()
				&& ch != '>' && ch != '='
				&& ch != '"' && ch != '\''
				&& ch != '`'
		})
		.to_string()
}

/// Take raw text up to the matching `</tag>`, splitting `{{expr}}` blocks when
/// enabled, appending tokens (but not the close tag, which the next loop emits).
fn parse_raw_text<'a>(
	cursor: &mut Cursor<'a>,
	tag: &str,
	config: &BsxFragmentConfig,
	tokens: &mut Vec<BsxFragmentToken<'a>>,
) {
	let mut text_start = cursor.offset();
	while !cursor.is_eof() {
		if starts_with_close_tag(cursor.rest(), tag) {
			break;
		}
		if config.raw_text_expressions && cursor.starts_with("{{") {
			let run = cursor.slice(text_start, cursor.offset());
			if !run.is_empty() {
				tokens.push(BsxFragmentToken::Text(run));
			}
			cursor.eat("{{");
			let expr = cursor.take_until("}}");
			cursor.eat("}}");
			tokens.push(BsxFragmentToken::Expr(expr));
			text_start = cursor.offset();
			continue;
		}
		cursor.bump();
	}
	let run = cursor.slice(text_start, cursor.offset());
	if !run.is_empty() {
		tokens.push(BsxFragmentToken::Text(run));
	}
}

/// Whether `input` starts with a `</tag>` close tag (case-insensitive).
fn starts_with_close_tag(input: &str, tag: &str) -> bool {
	let Some(rest) = input.strip_prefix("</") else {
		return false;
	};
	let rest = rest.trim_start();
	let Some(after) = rest
		.get(..tag.len())
		.filter(|candidate| candidate.eq_ignore_ascii_case(tag))
		.map(|_| &rest[tag.len()..])
	else {
		return false;
	};
	after.is_empty()
		|| after.starts_with('>')
		|| after.starts_with(char::is_whitespace)
}

/// Parse a run of sibling nodes until EOF or the matching `</close_tag>`.
fn parse_nodes(
	cursor: &mut Cursor,
	config: &BsxParseConfig,
	close_tag: Option<&str>,
) -> Result<Vec<BsxNode>> {
	let mut nodes = Vec::new();
	loop {
		if cursor.is_eof() {
			break;
		}
		// a closing tag for our parent ends this run.
		if let Some(tag) = close_tag {
			if cursor.starts_with("</") {
				let mut probe = Cursor::new(cursor.rest());
				probe.eat("</");
				probe.skip_ws();
				let name = probe.take_while(is_tag_char);
				if name == tag {
					break;
				}
			}
		}
		if cursor.starts_with("</") {
			// a stray closing tag with no open: stop, let the caller handle it.
			break;
		}
		if let Some(node) = parse_node(cursor, config)? {
			nodes.push(node);
		}
	}
	Ok(nodes)
}

/// Parse a single node: comment, doctype, element, `{..}` block, or text.
fn parse_node(
	cursor: &mut Cursor,
	config: &BsxParseConfig,
) -> Result<Option<BsxNode>> {
	if cursor.starts_with("<!--") {
		cursor.eat("<!--");
		let content = cursor.take_until("-->");
		cursor.eat("-->");
		return Ok(Some(BsxNode::Comment(content.to_string())));
	}
	if cursor.starts_with("<!") {
		cursor.eat("<!");
		// drop the leading `DOCTYPE` keyword, keep the value.
		let raw = cursor.take_until(">");
		cursor.eat(">");
		let value = raw
			.trim()
			.strip_prefix("DOCTYPE")
			.or_else(|| raw.trim().strip_prefix("doctype"))
			.unwrap_or(raw)
			.trim()
			.to_string();
		return Ok(Some(BsxNode::Doctype(value)));
	}
	if cursor.starts_with("<") {
		return parse_element(cursor, config).map(Some);
	}
	// a text-position `{..}` block, only in BSX mode.
	if config.bsx && cursor.peek() == Some('{') {
		return parse_text_block(cursor).map(Some);
	}
	parse_text(cursor, config)
}

/// Parse prose text up to the next `<` (and, in BSX mode, the next unescaped
/// `{`). Returns `None` for empty runs so resolution never spawns blank text.
fn parse_text(
	cursor: &mut Cursor,
	config: &BsxParseConfig,
) -> Result<Option<BsxNode>> {
	let text = cursor.take_while(|ch| {
		ch != '<' && !(config.bsx && ch == '{')
	});
	if text.is_empty() {
		// avoid an infinite loop on a stray char we did not consume.
		if !cursor.is_eof() {
			cursor.bump();
		}
		return Ok(None);
	}
	// decode character references in prose text (`&amp;` -> `&`); valid in both
	// XML-inspired BSX and HTML, and required for the renderer round-trip.
	Ok(Some(BsxNode::Text(decode_entities(&text))))
}

/// Decode the standard XML/HTML named character references plus numeric
/// (`&#NN;` / `&#xHH;`) references in prose text. An unrecognized `&name;` is
/// left verbatim, matching lenient HTML behavior.
fn decode_entities(input: &str) -> String {
	// fast path: no references present.
	if !input.contains('&') {
		return input.to_string();
	}
	let mut out = String::with_capacity(input.len());
	let mut rest = input;
	while let Some(amp) = rest.find('&') {
		out.push_str(&rest[..amp]);
		rest = &rest[amp..];
		match rest.find(';') {
			Some(semi) => {
				let entity = &rest[..=semi];
				match decode_one_entity(&entity[1..entity.len() - 1]) {
					Some(decoded) => out.push_str(&decoded),
					// unknown reference: keep it verbatim.
					None => out.push_str(entity),
				}
				rest = &rest[semi + 1..];
			}
			// no closing `;`: copy the remainder literally.
			None => {
				out.push_str(rest);
				return out;
			}
		}
	}
	out.push_str(rest);
	out
}

/// Decode one entity body (the text between `&` and `;`), or `None` if unknown.
fn decode_one_entity(body: &str) -> Option<String> {
	// numeric references: `&#NN;` (decimal) and `&#xHH;` (hex).
	if let Some(num) = body.strip_prefix('#') {
		let code = match num.strip_prefix(['x', 'X']) {
			Some(hex) => u32::from_str_radix(hex, 16).ok()?,
			None => num.parse::<u32>().ok()?,
		};
		return char::from_u32(code).map(String::from);
	}
	// the structural XML5 named references, the ones the round-trip relies on.
	let ch = match body {
		"amp" => '&',
		"lt" => '<',
		"gt" => '>',
		"quot" => '"',
		"apos" => '\'',
		"nbsp" => '\u{00A0}',
		_ => return None,
	};
	Some(String::from(ch))
}

/// Parse a text-position `{..}` block into a value expression.
fn parse_text_block(cursor: &mut Cursor) -> Result<BsxNode> {
	let inner = take_braced(cursor)?;
	let expr = parse_value_expr(&mut Cursor::new(&inner))?;
	Ok(BsxNode::Expr(expr))
}

/// Parse a `<tag ..>..</tag>` or `<tag ../>` element.
fn parse_element(
	cursor: &mut Cursor,
	config: &BsxParseConfig,
) -> Result<BsxNode> {
	cursor.eat("<");
	cursor.skip_ws();
	let tag = cursor.take_while(is_tag_char).to_string();
	if tag.is_empty() {
		bevybail!("expected a tag name after `<`");
	}
	let attributes = parse_attributes(cursor, config)?;
	cursor.skip_ws();
	let self_closing = cursor.eat("/>");
	if !self_closing && !cursor.eat(">") {
		bevybail!("expected `>` or `/>` to close opening tag `<{tag}>`");
	}

	let void = VOID_ELEMENTS.contains(&tag.as_str());
	if self_closing || void {
		return Ok(BsxNode::Element(BsxElement {
			tag,
			attributes,
			children: Vec::new(),
			self_closing: true,
		}));
	}

	// raw-text elements (script/style) take their content verbatim.
	let children = if RAW_TEXT_ELEMENTS.contains(&tag.as_str()) {
		let close = format!("</{tag}");
		let raw = cursor.take_until(&close);
		let mut kids = Vec::new();
		if !raw.is_empty() {
			kids.push(BsxNode::Text(raw.to_string()));
		}
		kids
	} else {
		parse_nodes(cursor, config, Some(&tag))?
	};

	// consume the matching close tag if present.
	if cursor.starts_with("</") {
		cursor.eat("</");
		cursor.skip_ws();
		cursor.take_while(is_tag_char);
		cursor.skip_ws();
		cursor.eat(">");
	}

	Ok(BsxNode::Element(BsxElement {
		tag,
		attributes,
		children,
		self_closing: false,
	}))
}

/// Parse the attribute list of an opening tag up to `>` or `/>`.
fn parse_attributes(
	cursor: &mut Cursor,
	config: &BsxParseConfig,
) -> Result<Vec<BsxAttribute>> {
	let mut attributes = Vec::new();
	loop {
		cursor.skip_ws();
		match cursor.peek() {
			None => break,
			Some('>') => break,
			Some('/') if cursor.starts_with("/>") => break,
			// a bare-position spread `<el {..}>`, BSX only.
			Some('{') if config.bsx => {
				let inner = take_braced(cursor)?;
				let spread = parse_spread(&mut Cursor::new(&inner))?;
				attributes.push(BsxAttribute {
					key: String::new(),
					value: AttrValue::Spread(spread),
				});
			}
			Some('{') => bevybail!("`{{..}}` spreads require bsx to be enabled"),
			_ => attributes.push(parse_attribute(cursor, config)?),
		}
	}
	Ok(attributes)
}

/// Parse a single `key`, `key="value"`, or (BSX) `key={..}` / `key=#foo`.
fn parse_attribute(
	cursor: &mut Cursor,
	config: &BsxParseConfig,
) -> Result<BsxAttribute> {
	let key = cursor.take_while(is_attr_key_char).to_string();
	if key.is_empty() {
		bevybail!("expected an attribute name");
	}
	if !config.bsx && key.starts_with("bx:") {
		bevybail!("`bx:` directives require bsx to be enabled");
	}
	cursor.skip_ws();
	if !cursor.eat("=") {
		return Ok(BsxAttribute {
			key,
			value: AttrValue::Flag,
		});
	}
	cursor.skip_ws();
	let value = match cursor.peek() {
		Some('"') => AttrValue::Str(parse_attr_string(cursor)?),
		Some('\'') => AttrValue::Str(parse_attr_string(cursor)?),
		Some('{') if config.bsx => {
			let inner = take_braced(cursor)?;
			AttrValue::Expr(parse_value_expr(&mut Cursor::new(&inner))?)
		}
		// unbraced value grammar, BSX only: `value=#foo=42`, `align=Center`.
		_ if config.bsx => {
			let raw = cursor.take_while(is_unbraced_value_char);
			AttrValue::Expr(parse_value_expr(&mut Cursor::new(raw))?)
		}
		_ => bevybail!("attribute `{key}` requires a quoted value in html mode"),
	};
	Ok(BsxAttribute { key, value })
}

/// Parse a quoted attribute string (single or double quotes).
fn parse_attr_string(cursor: &mut Cursor) -> Result<String> {
	let quote = cursor.bump().unwrap();
	let mut out = String::new();
	while let Some(ch) = cursor.bump() {
		if ch == quote {
			return Ok(out);
		}
		out.push(ch);
	}
	bevybail!("unterminated attribute string")
}

/// Consume a balanced `{..}` block, returning the inner text (braces stripped).
fn take_braced(cursor: &mut Cursor) -> Result<String> {
	cursor.eat("{");
	let mut depth = 1usize;
	let mut out = String::new();
	while let Some(ch) = cursor.bump() {
		match ch {
			'{' => {
				depth += 1;
				out.push(ch);
			}
			'}' => {
				depth -= 1;
				if depth == 0 {
					return Ok(out);
				}
				out.push(ch);
			}
			'"' => {
				// skip a quoted string so a `}` inside it does not close the block.
				out.push(ch);
				while let Some(inner) = cursor.bump() {
					out.push(inner);
					if inner == '"' {
						break;
					}
				}
			}
			other => out.push(other),
		}
	}
	bevybail!("unterminated `{{..}}` block")
}

/// Whether `ch` is valid in a tag name (incl `::` for `<path::to::X>`).
fn is_tag_char(ch: char) -> bool {
	ch.is_alphanumeric() || ch == '-' || ch == '_' || ch == ':'
}

/// Whether `ch` is valid in an attribute key (incl `:` for `bx:scope`).
fn is_attr_key_char(ch: char) -> bool {
	ch.is_alphanumeric() || ch == '-' || ch == '_' || ch == ':'
}

/// Whether `ch` continues an unbraced attribute value.
fn is_unbraced_value_char(ch: char) -> bool {
	!ch.is_whitespace() && ch != '>' && ch != '/'
}

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	fn simple_element() {
		let nodes = parse_document("<div>hi</div>", &BsxParseConfig::bsx())
			.unwrap();
		let BsxNode::Element(el) = &nodes[0] else {
			panic!("expected element");
		};
		el.tag.clone().xpect_eq("div".to_string());
		el.children.len().xpect_eq(1);
	}

	#[beet_core::test]
	fn nested_and_text() {
		let nodes = parse_document(
			"<div><span>inner</span></div>",
			&BsxParseConfig::bsx(),
		)
		.unwrap();
		let BsxNode::Element(div) = &nodes[0] else {
			panic!("expected div");
		};
		let BsxNode::Element(span) = &div.children[0] else {
			panic!("expected span");
		};
		span.tag.clone().xpect_eq("span".to_string());
	}

	#[beet_core::test]
	fn attributes() {
		let nodes =
			parse_document("<div class=\"card\" disabled/>", &BsxParseConfig::bsx())
				.unwrap();
		let BsxNode::Element(el) = &nodes[0] else {
			panic!("expected element");
		};
		el.attributes.len().xpect_eq(2);
		el.attributes[0]
			.value
			.clone()
			.xpect_eq(AttrValue::Str("card".to_string()));
		el.attributes[1].value.clone().xpect_eq(AttrValue::Flag);
	}

	#[beet_core::test]
	fn void_element() {
		let nodes =
			parse_document("<br>after", &BsxParseConfig::bsx()).unwrap();
		let BsxNode::Element(el) = &nodes[0] else {
			panic!("expected br");
		};
		el.children.is_empty().xpect_true();
		matches!(nodes[1], BsxNode::Text(_)).xpect_true();
	}

	#[beet_core::test]
	fn text_block_expr() {
		let nodes =
			parse_document("<p>{#count}</p>", &BsxParseConfig::bsx()).unwrap();
		let BsxNode::Element(el) = &nodes[0] else {
			panic!("expected p");
		};
		matches!(el.children[0], BsxNode::Expr(_)).xpect_true();
	}

	#[beet_core::test]
	fn comment_and_doctype() {
		let nodes = parse_document(
			"<!DOCTYPE html><!-- hi -->",
			&BsxParseConfig::bsx(),
		)
		.unwrap();
		nodes[0].clone().xpect_eq(BsxNode::Doctype("html".to_string()));
		nodes[1].clone().xpect_eq(BsxNode::Comment(" hi ".to_string()));
	}

	#[beet_core::test]
	fn html_mode_rejects_braces() {
		parse_document("<div {Foo}/>", &BsxParseConfig::html()).xpect_err();
	}

	#[beet_core::test]
	fn html_mode_rejects_bx() {
		parse_document("<div bx:scope=\"x\"/>", &BsxParseConfig::html())
			.xpect_err();
	}

	#[beet_core::test]
	fn html_mode_plain_html() {
		let nodes =
			parse_document("<div class=\"a\">hi</div>", &BsxParseConfig::html())
				.unwrap();
		let BsxNode::Element(el) = &nodes[0] else {
			panic!("expected div");
		};
		el.attributes[0]
			.value
			.clone()
			.xpect_eq(AttrValue::Str("a".to_string()));
	}

	// -- fragment primitive --

	fn fragment(source: &str) -> Vec<BsxFragmentToken<'_>> {
		parse_fragment(source, &BsxFragmentConfig::default()).unwrap()
	}

	#[beet_core::test]
	fn fragment_open_close() {
		fragment("<div>hi</div>").xpect_eq(vec![
			BsxFragmentToken::Open {
				tag: "div",
				attributes: vec![],
				self_closing: false,
			},
			BsxFragmentToken::Text("hi"),
			BsxFragmentToken::Close { tag: "div" },
		]);
	}

	#[beet_core::test]
	fn fragment_lone_open_is_safe() {
		// the key fragment property: an unbalanced open tag is a single token, not
		// an error, so a markdown builder can pair it with a later close event.
		fragment("<span>").xpect_eq(vec![BsxFragmentToken::Open {
			tag: "span",
			attributes: vec![],
			self_closing: false,
		}]);
		fragment("</span>")
			.xpect_eq(vec![BsxFragmentToken::Close { tag: "span" }]);
	}

	#[beet_core::test]
	fn fragment_void_and_self_closing() {
		matches!(
			fragment("<br>")[0],
			BsxFragmentToken::Open {
				self_closing: true,
				..
			}
		)
		.xpect_true();
		matches!(
			fragment("<br/>")[0],
			BsxFragmentToken::Open {
				self_closing: true,
				..
			}
		)
		.xpect_true();
	}

	#[beet_core::test]
	fn fragment_quoted_and_flag_attrs() {
		let BsxFragmentToken::Open { attributes, .. } =
			&fragment("<iframe src=\"x\" allowfullscreen>")[0]
		else {
			panic!("expected open");
		};
		attributes[0]
			.value
			.clone()
			.xpect_eq(AttrValue::Str("x".to_string()));
		attributes[1].value.clone().xpect_eq(AttrValue::Flag);
	}

	#[beet_core::test]
	fn fragment_unquoted_value_is_plain_string() {
		// an unquoted HTML URL is a string, not a value-grammar enum, and its
		// trailing slash survives (the `/` does not self-close the tag).
		let BsxFragmentToken::Open { attributes, .. } =
			&fragment("<meta content=https://bevy.org//>")[0]
		else {
			panic!("expected open");
		};
		attributes[0]
			.value
			.clone()
			.xpect_eq(AttrValue::Str("https://bevy.org//".to_string()));
	}

	#[beet_core::test]
	fn fragment_braced_attr_uses_value_grammar() {
		let BsxFragmentToken::Open { attributes, .. } =
			&fragment("<button bx:click={count.increment}>")[0]
		else {
			panic!("expected open");
		};
		matches!(attributes[0].value, AttrValue::Expr(_)).xpect_true();
	}

	#[beet_core::test]
	fn fragment_spread() {
		let BsxFragmentToken::Open { attributes, .. } =
			&fragment("<el {MyComponent}>")[0]
		else {
			panic!("expected open");
		};
		matches!(attributes[0].value, AttrValue::Spread(_)).xpect_true();
	}

	#[beet_core::test]
	fn fragment_comment_and_doctype() {
		fragment("<!DOCTYPE html><!-- hi -->").xpect_eq(vec![
			BsxFragmentToken::Doctype("html"),
			BsxFragmentToken::Comment(" hi "),
		]);
	}

	#[beet_core::test]
	fn fragment_script_raw_text() {
		// `<` inside raw text is not a tag, so the script body stays one text run.
		parse_fragment(
			"<script>let x = 1 < 2;</script>",
			&BsxFragmentConfig::default(),
		)
		.unwrap()
		.xpect_eq(vec![
			BsxFragmentToken::Open {
				tag: "script",
				attributes: vec![],
				self_closing: false,
			},
			BsxFragmentToken::Text("let x = 1 < 2;"),
			BsxFragmentToken::Close { tag: "script" },
		]);
	}

	#[beet_core::test]
	fn fragment_expressions() {
		let config = BsxFragmentConfig {
			expressions: true,
			..Default::default()
		};
		parse_fragment("<p>hi {name}</p>", &config).unwrap().xpect_eq(
			vec![
				BsxFragmentToken::Open {
					tag: "p",
					attributes: vec![],
					self_closing: false,
				},
				BsxFragmentToken::Text("hi "),
				BsxFragmentToken::Expr("name"),
				BsxFragmentToken::Close { tag: "p" },
			],
		);
	}

	#[beet_core::test]
	fn fragment_raw_text_expressions() {
		let config = BsxFragmentConfig {
			raw_text_expressions: true,
			..Default::default()
		};
		parse_fragment("<style>a {{x}} b</style>", &config)
			.unwrap()
			.xpect_eq(vec![
				BsxFragmentToken::Open {
					tag: "style",
					attributes: vec![],
					self_closing: false,
				},
				BsxFragmentToken::Text("a "),
				BsxFragmentToken::Expr("x"),
				BsxFragmentToken::Text(" b"),
				BsxFragmentToken::Close { tag: "style" },
			]);
	}
}
