//! The hand-written recursive-descent markup parser.
//!
//! One grammar: BSX with its extra surface enabled, HTML with it disabled.
//! [`BsxParseConfig::bsx`] gates the value grammar, `{..}` blocks, and
//! `bx:` directives, so the HTML-only mode is a real, tested configuration. The
//! parser produces a [`BsxNode`] tree; resolution into the world is
//! [`crate::bsx::resolve`].

use super::ast::*;
use super::cursor::Cursor;
use super::value::*;
use crate::bsx::resolve::is_event_directive;
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
pub(super) const VOID_ELEMENTS: &[&str] = &[
	"area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta",
	"param", "source", "track", "wbr",
];

/// Elements whose content is raw text up to their closing tag.
const RAW_TEXT_ELEMENTS: &[&str] = &["script", "style"];

impl BsxNode {
	/// Parse a full document into a flat list of root nodes.
	pub fn parse_document(
		source: &str,
		config: &BsxParseConfig,
	) -> Result<Vec<BsxNode>> {
		let mut cursor = Cursor::new(source);
		let mut nodes = parse_nodes(&mut cursor, config, None)?;
		normalize_whitespace(&mut nodes);
		Ok(nodes)
	}
}

/// Drop insignificant inter-element whitespace from a node tree, the runtime BSX
/// twin of the `rsx!` macro's compile-time trim and the browser's collapsing.
///
/// A whitespace-only [`BsxNode::Text`] is insignificant when both its neighbours
/// (or a list edge) are non-inline (an element, comment, or doctype): formatting
/// indentation between block tags. Whitespace touching inline content (text or an
/// `{expr}`) stays, collapsing to a single space at render. Recurses, but skips
/// `<pre>`-family elements whose whitespace is meaningful.
fn normalize_whitespace(nodes: &mut Vec<BsxNode>) {
	// a text/`{expr}` neighbour makes adjacent whitespace inline-significant.
	let is_inline = |node: &BsxNode| {
		matches!(node, BsxNode::Text(text) if !text.trim().is_empty())
			|| matches!(node, BsxNode::Expr(_))
	};
	// an insignificant node is a whitespace-only text run with no inline neighbour
	// (between block tags or comments), so it drops like the browser collapses it.
	let keep: Vec<bool> = nodes
		.iter()
		.enumerate()
		.map(|(index, node)| {
			let insignificant = matches!(node, BsxNode::Text(text) if text.trim().is_empty())
				&& index
					.checked_sub(1)
					.and_then(|prev| nodes.get(prev))
					.map_or(true, |prev| !is_inline(prev))
				&& nodes.get(index + 1).map_or(true, |next| !is_inline(next));
			!insignificant
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
	let text = cursor.take_while(|ch| ch != '<' && !(config.bsx && ch == '{'));
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
	// a tag-position component literal (`<Name("x")/>`, `<LogLike::Message("hi")/>`,
	// `<Greet{name:"world"}/>`) parses with the same sub-parser as a `{..}` spread,
	// building the component from the literal. The element's `tag` becomes the base
	// segment so the registry classifies the component; the close tag matches it.
	let (tag, tag_literal) = parse_tag_literal(cursor, tag, config)?;
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
			tag_literal,
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
		tag_literal,
		attributes,
		children,
		self_closing: false,
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
	if !config.bsx || !uppercase || !(tag.contains("::") || opens_fields) {
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
			Some('{') if config.bsx => {
				let inner = take_braced(cursor)?;
				let spread = parse_spread(&mut Cursor::new(&inner))?;
				attributes.push(BsxAttribute {
					key: String::new(),
					value: AttrValue::Spread(spread),
				});
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
	// the source position of the key, mapped onto the minted inline class for a
	// `bx:style` directive (the markup twin of `inline_class!`'s callsite).
	let key_offset = cursor.offset();
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
	// `bx:style="prop=value .."` declares a one-off rule; its value is parsed
	// downstream where the style types live, so keep the raw text plus the source
	// span (for a stable inline class).
	if config.bsx && key == "bx:style" {
		let source = parse_attr_string(cursor)?;
		return Ok(BsxAttribute {
			key,
			value: AttrValue::Style {
				source: source.into(),
				span: FileSpan::new(
					SmolPath::new("bsx"),
					cursor.line_col(key_offset),
					cursor.line_col(cursor.offset()),
				),
			},
		});
	}
	// a `bx:<event>` directive is a verb call `increment{ field: @doc:count }`,
	// parsed straight off the cursor so its internal whitespace survives.
	if config.bsx && is_event_directive(&key) {
		let verb = parse_verb_call(cursor)?;
		return Ok(BsxAttribute {
			key,
			value: AttrValue::Verb(verb),
		});
	}
	let value = match cursor.peek() {
		Some('"') => AttrValue::Str(parse_attr_string(cursor)?),
		Some('\'') => AttrValue::Str(parse_attr_string(cursor)?),
		Some('{') if config.bsx => {
			let inner = take_braced(cursor)?;
			AttrValue::Expr(parse_value_expr(&mut Cursor::new(&inner))?)
		}
		// unbraced value grammar, BSX only: `value=@doc:foo=42`, `align=Center`.
		_ if config.bsx => {
			let raw = cursor.take_while(is_unbraced_value_char);
			AttrValue::Expr(parse_value_expr(&mut Cursor::new(raw))?)
		}
		_ => {
			bevybail!("attribute `{key}` requires a quoted value in html mode")
		}
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
pub(super) fn take_braced(cursor: &mut Cursor) -> Result<String> {
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
pub(super) fn is_tag_char(ch: char) -> bool {
	ch.is_alphanumeric() || ch == '-' || ch == '_' || ch == ':'
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

	#[crate::test]
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

	#[crate::test]
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

	#[crate::test]
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

	#[crate::test]
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

	#[crate::test]
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

	#[crate::test]
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
		// a lowercase or `path::to::X` tag is never a tag-position literal.
		element("<div/>").tag_literal.xpect_none();
		element("<path::to::X/>").tag_literal.xpect_none();
	}

	#[crate::test]
	fn comment_and_doctype() {
		let nodes = BsxNode::parse_document(
			"<!DOCTYPE html><!-- hi -->",
			&BsxParseConfig::bsx(),
		)
		.unwrap();
		nodes[0]
			.clone()
			.xpect_eq(BsxNode::Doctype("html".to_string()));
		nodes[1]
			.clone()
			.xpect_eq(BsxNode::Comment(" hi ".to_string()));
	}

	#[crate::test]
	fn html_mode_rejects_braces() {
		BsxNode::parse_document("<div {Foo}/>", &BsxParseConfig::html())
			.xpect_err();
	}

	#[crate::test]
	fn html_mode_rejects_bx() {
		BsxNode::parse_document(
			"<div bx:scope=\"x\"/>",
			&BsxParseConfig::html(),
		)
		.xpect_err();
	}

	#[crate::test]
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
