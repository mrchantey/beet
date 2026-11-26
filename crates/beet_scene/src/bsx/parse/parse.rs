//! The hand-written recursive-descent markup parser.
//!
//! One grammar in three dialects: BSX with its extra surface enabled, HTML with
//! it disabled, and XML, which is HTML's markup read as written.
//! [`BsxParseConfig`] gates the value grammar, `{..}` blocks, and `bx:`
//! directives, so the HTML-only and XML modes are real, tested configurations.
//! The parser produces a [`BsxNode`] tree, each node with the range it was
//! written at, which [`BsxParser::lower`] takes from there.

use super::ast::*;
use super::cursor::Cursor;
use super::span::*;
use super::value::*;
#[cfg(doc)]
use crate::prelude::BsxParser;
use beet_core::prelude::*;

/// Configuration selecting the dialect a markup source is read in.
#[derive(Debug, Default, Clone)]
pub struct BsxParseConfig {
	/// The dialect.
	pub dialect: MarkupDialect,
}

/// The dialects of the one markup grammar.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum MarkupDialect {
	/// The full BSX grammar: the value grammar (`{..}` literals and
	/// references, bare spreads) and `bx:` directives.
	#[default]
	Bsx,
	/// HTML: the BSX surface disabled, so lowercase tags, string attributes,
	/// text and comments, with void and raw text elements as HTML has them.
	Html,
	/// XML: HTML's markup read exactly as written. Tags keep their case and
	/// may carry a `.`, no element is void or raw text, whitespace is kept
	/// wherever it falls, a literal tab or line break in an attribute value
	/// is the space the specification normalizes it to, and a closing tag
	/// must close the element it names.
	Xml,
}

impl BsxParseConfig {
	/// The full BSX grammar.
	pub fn bsx() -> Self {
		Self {
			dialect: MarkupDialect::Bsx,
		}
	}
	/// HTML-only: the BSX surface disabled.
	pub fn html() -> Self {
		Self {
			dialect: MarkupDialect::Html,
		}
	}
	/// XML: markup as written, see [`MarkupDialect::Xml`].
	pub fn xml() -> Self {
		Self {
			dialect: MarkupDialect::Xml,
		}
	}
	/// The media types a dialect reads, one per dialect.
	pub const MEDIA_TYPES: [MediaType; 3] =
		[MediaType::Bsx, MediaType::Html, MediaType::Xml];

	/// The dialect `media_type` is read in, `None` for a non-markup type.
	pub fn from_media_type(media_type: &MediaType) -> Option<Self> {
		match media_type {
			MediaType::Bsx => Some(Self::bsx()),
			MediaType::Html => Some(Self::html()),
			MediaType::Xml => Some(Self::xml()),
			_ => None,
		}
	}
	/// Whether the BSX surface is enabled.
	pub fn is_bsx(&self) -> bool { self.dialect == MarkupDialect::Bsx }
	/// Whether markup is read as XML.
	pub fn is_xml(&self) -> bool { self.dialect == MarkupDialect::Xml }
}

/// Refuse a tag naming a reserved word in another's spelling, with the
/// spelling to use, rather than hold it as an unregistered tag: a slot
/// placeholder is HTML's lowercase `<slot>`, and another document is an
/// [`Instance`].
pub(super) fn refuse_reserved_tag(tag: &str) -> Result {
	match tag {
		"Template" => bevybail!(
			"`<Template>` is no tag: another document is \
			 `<Instance document=\"./other.bsx\"/>`, and a prefab is HTML's \
			 `<template>`"
		),
		"Slot" => bevybail!(
			"`<Slot>` is spelled `<slot>`, HTML's own element: `<slot/>` for \
			 the default slot, `<slot name=\"..\"/>` for a named one"
		),
		_ => Ok(()),
	}
}


impl BsxNode {
	/// Parse a full document into a flat list of root nodes.
	pub fn parse_document(
		source: &str,
		config: &BsxParseConfig,
	) -> Result<Vec<BsxNode>> {
		let mut cursor = Cursor::new(source);
		// a byte order mark announces the encoding, it is no content
		cursor.eat("\u{FEFF}");
		let nodes = parse_nodes(&mut cursor, config, None);
		let mut nodes = cursor.locate(nodes)?;
		match config.is_xml() {
			// xml keeps every character, and a stray closing tag is malformed
			true if !cursor.is_eof() => {
				return cursor.locate(Err(bevyhow!(
					"a closing tag at {} closes no open element",
					cursor.offset()
				)));
			}
			true => {}
			false => normalize_whitespace(&mut nodes),
		}
		Ok(nodes)
	}
}

/// Drop insignificant inter-element whitespace from a node tree, the runtime BSX
/// twin of the `rsx!` macro's compile-time trim and the browser's collapsing.
///
/// A whitespace-only [`BsxNode::Text`] is insignificant when neither neighbour
/// (or a list edge) is inline content (text or an `{expr}`): formatting
/// indentation between tags. Whitespace touching inline content stays,
/// collapsing to a single space at render, as does a run on one line between
/// two [`Element::INLINE_ELEMENTS`] tags, ie `<b>a</b> <i>b</i>`, the way JSX
/// keeps a space but not a line break. Recurses, but skips `<pre>`-family
/// elements whose whitespace is meaningful.
fn normalize_whitespace(nodes: &mut Vec<BsxNode>) {
	// a text/`{expr}` neighbour makes adjacent whitespace inline-significant.
	let is_inline = |node: &BsxNode| {
		matches!(node, BsxNode::Text(text) if !text.trim().is_empty())
			|| matches!(node, BsxNode::Expr(_))
	};
	let is_inline_tag = |node: &BsxNode| {
		matches!(node, BsxNode::Element(element)
			if Element::INLINE_ELEMENTS.contains(&element.tag.as_str()))
	};
	// an insignificant node is a whitespace-only text run with no inline neighbour
	// (between block tags or comments), so it drops like the browser collapses it.
	let keep: Vec<bool> = nodes
		.iter()
		.enumerate()
		.map(|(index, node)| {
			let prev = index.checked_sub(1).and_then(|prev| nodes.get(prev));
			let next = nodes.get(index + 1);
			let BsxNode::Text(text) = node else {
				return true;
			};
			let word_break = !text.contains('\n')
				&& prev.is_some_and(is_inline_tag)
				&& next.is_some_and(is_inline_tag);
			!text.trim().is_empty()
				|| prev.is_some_and(is_inline)
				|| next.is_some_and(is_inline)
				|| word_break
		})
		.collect();
	let mut keep = keep.into_iter();
	nodes.retain(|_| keep.next().unwrap_or(true));
	// recurse into element children, leaving `<pre>`-family content verbatim.
	for node in nodes.iter_mut() {
		if let BsxNode::Element(element) = node {
			if !Element::PRE_ELEMENTS.contains(&element.tag.as_str()) {
				normalize_whitespace(&mut element.children);
			}
		}
	}
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
				let mut probe = cursor.nested(cursor.offset()..cursor.end());
				probe.eat("</");
				probe.skip_ws();
				let name = probe.take_while(|ch| is_name_char(ch, config));
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

/// Parse a single node: comment, CDATA section, processing instruction,
/// doctype, element, `{..}` block, or text.
fn parse_node(
	cursor: &mut Cursor,
	config: &BsxParseConfig,
) -> Result<Option<BsxNode>> {
	let start = cursor.offset();
	let spanned = |cursor: &Cursor, text: &str| {
		Spanned::new(text.to_string(), cursor.span(start))
	};
	if cursor.eat("<!--") {
		let content = cursor.take_until("-->");
		cursor.eat("-->");
		return Ok(Some(BsxNode::Comment(spanned(cursor, content))));
	}
	if cursor.eat("<![CDATA[") {
		let content = cursor.take_until("]]>");
		if !cursor.eat("]]>") {
			bevybail!("unterminated `<![CDATA[..]]>` section");
		}
		return Ok(Some(BsxNode::CData(spanned(cursor, content))));
	}
	if cursor.eat("<?") {
		let content = cursor.take_until("?>");
		if !cursor.eat("?>") {
			bevybail!("unterminated `<?..?>` processing instruction");
		}
		return Ok(Some(BsxNode::ProcessingInstruction(spanned(
			cursor, content,
		))));
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
			.trim();
		return Ok(Some(BsxNode::Doctype(spanned(cursor, value))));
	}
	if cursor.starts_with("<") {
		return parse_element(cursor, config).map(Some);
	}
	// a text-position `{..}` block, only in BSX mode.
	if config.is_bsx() && cursor.peek() == Some('{') {
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
	let start = cursor.offset();
	let text =
		cursor.take_while(|ch| ch != '<' && !(config.is_bsx() && ch == '{'));
	if text.is_empty() {
		// avoid an infinite loop on a stray char we did not consume.
		if !cursor.is_eof() {
			cursor.bump();
		}
		return Ok(None);
	}
	// decode character references in prose text (`&amp;` -> `&`); valid in both
	// XML-inspired BSX and HTML, and required for the renderer round-trip.
	Ok(Some(BsxNode::Text(Spanned::new(
		html_ext::decode_references(text),
		cursor.span(start),
	))))
}

/// Parse a text-position `{..}` block into a value expression.
fn parse_text_block(cursor: &mut Cursor) -> Result<BsxNode> {
	let start = cursor.offset();
	let expr = take_braced(cursor)?.read(parse_value_expr)?;
	Ok(BsxNode::Expr(Spanned::new(expr, cursor.span(start))))
}

/// Parse a `<tag ..>..</tag>` or `<tag ../>` element.
fn parse_element(
	cursor: &mut Cursor,
	config: &BsxParseConfig,
) -> Result<BsxNode> {
	let start = cursor.offset();
	cursor.eat("<");
	cursor.skip_ws();
	let tag = cursor.take_while(|ch| is_name_char(ch, config)).to_string();
	if tag.is_empty() {
		bevybail!("expected a tag name after `<`");
	}
	// a tag-position component literal (`<Name("x")/>`, `<LogLike::Message("hi")/>`,
	// `<Greet{name:"world"}/>`) parses with the same sub-parser as a `{..}` spread,
	// building the component from the literal. The element's `tag` becomes the base
	// segment so the registry classifies the component; the close tag matches it.
	let (tag, tag_literal) = parse_tag_literal(cursor, tag, config)?;
	if config.is_bsx() {
		refuse_reserved_tag(&tag)
			.map_err(|error| SourceError::at(error, start))?;
	}
	let attributes = parse_attributes(cursor, config)?;
	cursor.skip_ws();
	let self_closing = cursor.eat("/>");
	if !self_closing && !cursor.eat(">") {
		bevybail!("expected `>` or `/>` to close opening tag `<{tag}>`");
	}

	// xml has no void elements: an element is empty only when written so
	let void =
		!config.is_xml() && html_ext::VOID_ELEMENTS.contains(&tag.as_str());
	if self_closing || void {
		return Ok(BsxNode::Element(BsxElement {
			tag,
			tag_literal,
			key: None,
			attributes,
			children: Vec::new(),
			self_closing: true,
			span: cursor.span(start),
		}));
	}

	// raw-text elements (script/style) take their content verbatim; xml has
	// none, a `<script>` there being markup like any other element.
	let children = if !config.is_xml()
		&& html_ext::RAW_TEXT_ELEMENTS.contains(&tag.as_str())
	{
		let close = format!("</{tag}");
		let raw_start = cursor.offset();
		let raw = cursor.take_until(&close);
		let mut kids = Vec::new();
		if !raw.is_empty() {
			kids.push(BsxNode::Text(Spanned::new(
				raw.to_string(),
				cursor.span(raw_start),
			)));
		}
		kids
	} else {
		parse_nodes(cursor, config, Some(&tag))?
	};

	// consume the matching close tag if present; xml requires it.
	if cursor.starts_with("</") {
		cursor.eat("</");
		cursor.skip_ws();
		let closed = cursor.take_while(|ch| is_name_char(ch, config));
		if config.is_xml() && closed != tag {
			bevybail!("`<{tag}>` is closed by `</{closed}>`");
		}
		cursor.skip_ws();
		cursor.eat(">");
	} else if config.is_xml() {
		return Err(SourceError::at(
			bevyhow!("`<{tag}>` is never closed"),
			start,
		));
	}

	Ok(BsxNode::Element(BsxElement {
		tag,
		tag_literal,
		key: None,
		attributes,
		children,
		self_closing: false,
		span: cursor.span(start),
	}))
}

/// Detect and parse a tag-position component literal, returning the base
/// component name and the literal. The `tag` was already read by the cursor; a
/// literal is present when its first char is uppercase and either the tag is
/// `::`-qualified (an enum variant, eg `LogLike::Message`) or the next char opens a
/// tuple/struct body (`(`/`{`). The literal's fields parse via the shared
/// [`parse_named_fields`]; the returned `tag` is the segment before `::` (so
/// `LogLike::Message` classifies as `LogLike`, `Name` stays `Name`), and the close tag
/// matches that base. Outside BSX mode (HTML), or for a plain tag, the tag is
/// returned unchanged with no literal.
fn parse_tag_literal(
	cursor: &mut Cursor,
	tag: String,
	config: &BsxParseConfig,
) -> Result<(String, Option<NamedLiteral>)> {
	let uppercase = tag.starts_with(|ch: char| ch.is_uppercase());
	let opens_fields = matches!(cursor.peek(), Some('(') | Some('{'));
	if !config.is_bsx() || !uppercase || !(tag.contains("::") || opens_fields) {
		return Ok((tag, None));
	}
	let fields = parse_named_fields(cursor)?;
	let base = tag.split("::").next().unwrap_or(&tag).to_string();
	let literal = NamedLiteral {
		name: tag.into(),
		fields,
	};
	Ok((base, Some(literal)))
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
			Some('{') if config.is_bsx() => {
				let start = cursor.offset();
				let spread = take_braced(cursor)?.read(parse_spread)?;
				attributes.push(
					BsxAttribute::spread(spread).with_span(cursor.span(start)),
				);
			}
			Some('{') => {
				bevybail!("`{{..}}` spreads require bsx to be enabled")
			}
			_ => attributes.push(parse_attribute(cursor, config)?),
		}
	}
	Ok(attributes)
}

/// Parse a single `key`, `key="value"`, or (BSX) `key={..}` / `key=@doc:foo`.
fn parse_attribute(
	cursor: &mut Cursor,
	config: &BsxParseConfig,
) -> Result<BsxAttribute> {
	let start = cursor.offset();
	let key = cursor
		.take_while(|ch| is_attr_key_char(ch) || (config.is_xml() && ch == '.'))
		.to_string();
	if key.is_empty() {
		bevybail!("expected an attribute name");
	}
	if !config.is_bsx() && key.starts_with("bx:") {
		bevybail!("`bx:` directives require bsx to be enabled");
	}
	cursor.skip_ws();
	if !cursor.eat("=") {
		return BsxAttribute::new(key, AttrValue::Flag)
			.with_span(cursor.span(start))
			.xok();
	}
	cursor.skip_ws();
	let value = match cursor.peek() {
		Some('"' | '\'') => {
			let written = parse_attr_string(cursor)?;
			AttrValue::Str(match config.dialect {
				// a bsx string is authored as is, its `&&` a script's
				MarkupDialect::Bsx => written,
				MarkupDialect::Html => html_ext::decode_references(&written),
				// a literal tab or line break is the space the specification
				// normalizes it to, while a reference to one, `&#xA;`, is kept
				MarkupDialect::Xml => html_ext::decode_references(
					&written.replace(['\t', '\n', '\r'], " "),
				),
			})
		}
		Some('{') if config.is_bsx() => {
			AttrValue::Expr(take_braced(cursor)?.read(parse_value_expr)?)
		}
		// unbraced value grammar, BSX only: `value=@doc:foo=42`, `align=Center`.
		_ if config.is_bsx() => {
			let start = cursor.offset();
			cursor.take_while(is_unbraced_value_char);
			AttrValue::Expr(
				cursor
					.nested(start..cursor.offset())
					.read(parse_value_expr)?,
			)
		}
		_ => {
			bevybail!("attribute `{key}` requires a quoted value in html mode")
		}
	};
	BsxAttribute::new(key, value)
		.with_span(cursor.span(start))
		.xok()
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

/// Consume a balanced `{..}` block, answering a cursor over the inner text
/// (braces stripped).
pub(super) fn take_braced<'a>(cursor: &mut Cursor<'a>) -> Result<Cursor<'a>> {
	let open = cursor.offset();
	cursor.eat("{");
	let start = cursor.offset();
	let mut depth = 1usize;
	while let Some(ch) = cursor.bump() {
		match ch {
			'{' => depth += 1,
			'}' => {
				depth -= 1;
				if depth == 0 {
					return cursor.nested(start..cursor.offset() - 1).xok();
				}
			}
			'"' => {
				// skip a quoted string so a `}` inside it does not close the block.
				while let Some(inner) = cursor.bump() {
					if inner == '"' {
						break;
					}
				}
			}
			_ => {}
		}
	}
	Err(SourceError::at(
		bevyhow!("unterminated `{{..}}` block"),
		open,
	))
}

/// Whether `ch` is valid in a tag name (incl `::` for `<path::to::X>`).
pub(super) fn is_tag_char(ch: char) -> bool {
	ch.is_alphanumeric() || ch == '-' || ch == '_' || ch == ':'
}

/// Whether `ch` continues an element name in `config`'s dialect: a tag char,
/// or a `.` in xml, ie `<x14ac.dyDescent>`.
fn is_name_char(ch: char, config: &BsxParseConfig) -> bool {
	is_tag_char(ch) || (config.is_xml() && ch == '.')
}

/// Whether `ch` is valid in an attribute key (incl `:` for `bx:scope`).
pub(super) fn is_attr_key_char(ch: char) -> bool {
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
		let nodes =
			BsxNode::parse_document("<div>hi</div>", &BsxParseConfig::bsx())
				.unwrap();
		let BsxNode::Element(el) = &nodes[0] else {
			panic!("expected element");
		};
		el.tag.clone().xpect_eq("div".to_string());
		el.children.len().xpect_eq(1);
	}

	#[beet_core::test]
	fn nested_and_text() {
		let nodes = BsxNode::parse_document(
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
		let nodes = BsxNode::parse_document(
			"<div class=\"card\" disabled/>",
			&BsxParseConfig::bsx(),
		)
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
			BsxNode::parse_document("<br>after", &BsxParseConfig::bsx())
				.unwrap();
		let BsxNode::Element(el) = &nodes[0] else {
			panic!("expected br");
		};
		el.children.is_empty().xpect_true();
		matches!(nodes[1], BsxNode::Text(_)).xpect_true();
	}

	#[beet_core::test]
	fn text_block_expr() {
		let nodes = BsxNode::parse_document(
			"<p>{@doc:count}</p>",
			&BsxParseConfig::bsx(),
		)
		.unwrap();
		let BsxNode::Element(el) = &nodes[0] else {
			panic!("expected p");
		};
		matches!(el.children[0], BsxNode::Expr(_)).xpect_true();
	}

	#[beet_core::test]
	fn tag_position_literal() {
		// a tuple/struct/enum tag-position literal parses into `tag_literal`, with
		// `tag` reduced to the base component name (the segment before `::`).
		let element = |source: &str| -> BsxElement {
			let nodes = BsxNode::parse_document(source, &BsxParseConfig::bsx())
				.unwrap();
			let BsxNode::Element(element) = nodes.into_iter().next().unwrap()
			else {
				panic!("expected an element");
			};
			element
		};
		// tuple form: base tag unchanged, literal carries the tuple field.
		let name = element("<Name(\"x\")/>");
		name.tag.clone().xpect_eq("Name".to_string());
		let literal = name.tag_literal.clone().unwrap();
		literal.name.as_str().xpect_eq("Name");
		matches!(literal.fields, NamedFields::Tuple(_)).xpect_true();
		// `::`-qualified enum variant: base tag is the segment before `::`.
		let log = element("<LogLike::Message(\"hi\")/>");
		log.tag.clone().xpect_eq("LogLike".to_string());
		log.tag_literal
			.unwrap()
			.name
			.as_str()
			.xpect_eq("LogLike::Message");
		// struct form.
		let cfg = element("<Greet{name:\"world\"}/>");
		cfg.tag.clone().xpect_eq("Greet".to_string());
		matches!(cfg.tag_literal.unwrap().fields, NamedFields::Struct(_))
			.xpect_true();
		// a bare uppercase tag is not a literal, so children resolve as a component.
		let bare = element("<Sequence>x</Sequence>");
		bare.tag.clone().xpect_eq("Sequence".to_string());
		bare.tag_literal.xpect_none();
		// a lowercase tag is never a tag-position literal.
		element("<div/>").tag_literal.xpect_none();
	}

	#[beet_core::test]
	fn comment_and_doctype() {
		let nodes = BsxNode::parse_document(
			"<!DOCTYPE html><!-- hi -->",
			&BsxParseConfig::bsx(),
		)
		.unwrap();
		nodes[0].clone().xpect_eq(BsxNode::Doctype("html".into()));
		nodes[1].clone().xpect_eq(BsxNode::Comment(" hi ".into()));
	}

	#[beet_core::test]
	fn html_mode_rejects_braces() {
		BsxNode::parse_document("<div {Foo}/>", &BsxParseConfig::html())
			.xpect_err();
	}

	#[beet_core::test]
	fn html_mode_rejects_bx() {
		BsxNode::parse_document(
			"<div bx:scope=\"x\"/>",
			&BsxParseConfig::html(),
		)
		.xpect_err();
	}

	/// XML keeps every character as written: whitespace between elements,
	/// references decoded in text and attributes, a literal line break in an
	/// attribute normalized to a space while a referenced one is kept, no
	/// void element, and CDATA and processing instructions as nodes.
	#[beet_core::test]
	fn xml_mode_reads_as_written() {
		let nodes = BsxNode::parse_document(
			"\u{FEFF}<?xml version=\"1.0\"?>\r\n<w:p a=\"x&#xA;y\" b=\"x\ny\" w14.c=\"1\">\n\t<br>one &amp; <Two/></br>\n<![CDATA[a < b]]></w:p>",
			&BsxParseConfig::xml(),
		)
		.unwrap();
		nodes[0].clone().xpect_eq(BsxNode::ProcessingInstruction(
			"xml version=\"1.0\"".into(),
		));
		nodes[1].clone().xpect_eq(BsxNode::Text("\r\n".into()));
		let BsxNode::Element(paragraph) = &nodes[2] else {
			panic!("expected w:p");
		};
		paragraph.tag.clone().xpect_eq("w:p".to_string());
		paragraph.attributes[0]
			.value
			.clone()
			.xpect_eq(AttrValue::Str("x\ny".into()));
		paragraph.attributes[1]
			.value
			.clone()
			.xpect_eq(AttrValue::Str("x y".into()));
		paragraph.attributes[2]
			.key
			.clone()
			.xpect_eq("w14.c".to_string());
		paragraph.children[0]
			.clone()
			.xpect_eq(BsxNode::Text("\n\t".into()));
		let BsxNode::Element(br) = &paragraph.children[1] else {
			panic!("expected br");
		};
		// `br` is no void element in xml, so it holds what it was written with
		br.children[0]
			.clone()
			.xpect_eq(BsxNode::Text("one & ".into()));
		paragraph.children[3]
			.clone()
			.xpect_eq(BsxNode::CData("a < b".into()));
	}

	#[beet_core::test]
	fn xml_mode_refuses_malformed_closes() {
		BsxNode::parse_document("<a><b></a>", &BsxParseConfig::xml())
			.unwrap_err()
			.to_string()
			.xpect_contains("`<b>` is closed by `</a>`");
		BsxNode::parse_document("<a>", &BsxParseConfig::xml())
			.unwrap_err()
			.to_string()
			.xpect_contains("never closed");
		BsxNode::parse_document("<a/></b>", &BsxParseConfig::xml()).xpect_err();
	}

	/// HTML decodes an attribute's references, so a renderer escaping it on
	/// the way out round-trips it.
	#[beet_core::test]
	fn html_mode_decodes_attributes() {
		let nodes = BsxNode::parse_document(
			"<a href=\"?a=1&amp;b=2\">x</a>",
			&BsxParseConfig::html(),
		)
		.unwrap();
		let BsxNode::Element(link) = &nodes[0] else {
			panic!("expected a");
		};
		link.attributes[0]
			.value
			.clone()
			.xpect_eq(AttrValue::Str("?a=1&b=2".into()));
	}

	#[beet_core::test]
	fn html_mode_plain_html() {
		let nodes = BsxNode::parse_document(
			"<div class=\"a\">hi</div>",
			&BsxParseConfig::html(),
		)
		.unwrap();
		let BsxNode::Element(el) = &nodes[0] else {
			panic!("expected div");
		};
		el.attributes[0]
			.value
			.clone()
			.xpect_eq(AttrValue::Str("a".to_string()));
	}
}
