//! The lenient fragment tokenizer: markup in, a flat token stream out.
//!
//! Unlike the document parser it never requires the fragment to balance, so a
//! stack-based caller (the markdown interleaving builder) can feed it one
//! `pulldown-cmark` HTML fragment at a time. It shares the cursor, the
//! attribute grammar and the value grammar with [`super::parse`], so embedded
//! markup resolves through the one parser.

use super::ast::*;
use super::cursor::Cursor;
use super::parse::VOID_ELEMENTS;
use super::parse::is_attr_key_char;
use super::parse::is_tag_char;
use super::parse::take_braced;
use super::value::*;
use crate::bsx::resolve::is_event_directive;
use crate::prelude::*;

/// One lenient, fragment-safe markup token produced by [`BsxFragmentToken::parse_fragment`].
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

/// Configuration for [`BsxFragmentToken::parse_fragment`], mirroring the markup quirks an embedded
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

impl<'a> BsxFragmentToken<'a> {
	/// Tokenize a markup fragment leniently into a flat [`BsxFragmentToken`] stream.
	///
	/// This is the fragment-level entry the markdown builder drives instead of its
	/// own HTML tokenizer: it reuses the BSX cursor, attribute parser, and value
	/// grammar, so embedded markup resolves through the one parser. It never
	/// requires the fragment to balance, a lone `<span>` or a lone `</span>` each
	/// produce a single token, because `pulldown-cmark` splits a tag and its close
	/// across separate events. It is no_std-clean (allocates only the attribute
	/// list, borrows every verbatim span from `source`).
	pub fn parse_fragment(
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
	let text =
		cursor.take_while(|ch| ch != '<' && !(config.expressions && ch == '{'));
	if text.is_empty() {
		// a stray char (eg a `<` not starting a valid tag): consume it as text so
		// the loop advances, matching lenient HTML.
		if !cursor.is_eof() {
			cursor.bump();
		}
		let consumed = cursor.slice(start, cursor.offset());
		return Ok(
			(!consumed.is_empty()).then_some(BsxFragmentToken::Text(consumed))
		);
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
fn parse_fragment_attributes(cursor: &mut Cursor) -> Result<Vec<BsxAttribute>> {
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
///
/// A fragment is always BSX (it already parses `{..}` spreads and expressions),
/// so the `bx:` directives are resolved here too, identically to the document
/// parser ([`parse_attribute`]): a `bx:style` keeps its raw text plus span for a
/// stable inline class, and a `bx:<event>` parses its verb call. Without this,
/// embedded markdown markup silently drops these directives (the `.md`/`.bsx`
/// parity gap surfaced by the no-code site rehearsal).
fn parse_fragment_attribute(cursor: &mut Cursor) -> Result<BsxAttribute> {
	let key_offset = cursor.offset();
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
	// `bx:style="prop=value .."` declares a one-off rule; its value is parsed
	// downstream where the style types live, so keep the raw text plus a source
	// span (for a stable inline class), mirroring the document parser.
	if key == "bx:style" {
		let source = parse_fragment_string(cursor);
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
	// a `bx:<event>=verb{ .. }` directive is a verb call, parsed off the cursor so
	// its internal whitespace survives. The braced form `bx:<event>={expr}` is a
	// plain expression, so it falls through to the `{` arm below.
	if is_event_directive(&key) && cursor.peek() != Some('{') {
		return Ok(BsxAttribute {
			key,
			value: AttrValue::Verb(parse_verb_call(cursor)?),
		});
	}
	let value = match cursor.peek() {
		Some('"') | Some('\'') => AttrValue::Str(parse_fragment_string(cursor)),
		Some('{') => {
			let inner = take_braced(cursor)?;
			AttrValue::Expr(parse_value_expr(&mut Cursor::new(&inner))?)
		}
		// unquoted HTML value: a plain string up to whitespace, `>` or `/>`. Per
		// the HTML spec `/` does not terminate it, so URLs survive.
		_ => fragment_unquoted_value(parse_unquoted_value(cursor)),
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

/// Classify an unquoted fragment attribute value, resolving the forms that a
/// component prop needs through the value grammar while keeping bare HTML values
/// as plain strings:
///
/// - a Rust **qualified path** (`::`, eg `variant=ButtonVariant::Filled`) reaches
///   a typed enum-variant prop;
/// - a **boolean** (`true`/`false`, eg `vertical_lines=true`) reaches a typed
///   `bool` prop, instead of the string `"true"` failing prop validation and
///   dropping the whole template.
///
/// Both mirror what the document parser and `rsx!` accept. Every other unquoted
/// value stays a plain string so HTML URLs survive verbatim (`src=https://x//`,
/// whose `://` is never a path separator). A value that fails to parse falls back
/// to a string, never an error.
fn fragment_unquoted_value(raw: String) -> AttrValue {
	if raw.contains("::") || raw == "true" || raw == "false" {
		if let Ok(expr) = parse_value_expr(&mut Cursor::new(&raw)) {
			return AttrValue::Expr(expr);
		}
	}
	AttrValue::Str(raw)
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
#[cfg(test)]
mod test {
	use super::*;

	fn fragment(source: &str) -> Vec<BsxFragmentToken<'_>> {
		BsxFragmentToken::parse_fragment(source, &BsxFragmentConfig::default())
			.unwrap()
	}

	#[crate::test]
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

	#[crate::test]
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

	#[crate::test]
	fn fragment_void_and_self_closing() {
		matches!(fragment("<br>")[0], BsxFragmentToken::Open {
			self_closing: true,
			..
		})
		.xpect_true();
		matches!(fragment("<br/>")[0], BsxFragmentToken::Open {
			self_closing: true,
			..
		})
		.xpect_true();
	}

	#[crate::test]
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

	#[crate::test]
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

	#[crate::test]
	fn fragment_unquoted_qualified_path_is_value_expr() {
		// an unquoted Rust qualified path (`::`) resolves through the value grammar
		// so an embedded-markup component prop like `variant=ButtonVariant::Filled`
		// reaches the typed enum, instead of staying the literal string (which
		// silently fell back to the enum default). A URL's `://` is untouched.
		let BsxFragmentToken::Open { attributes, .. } =
			&fragment("<Button variant=ButtonVariant::Outlined>")[0]
		else {
			panic!("expected open");
		};
		let AttrValue::Expr(ValueExpr::Literal(DataLiteral::Enum(named))) =
			&attributes[0].value
		else {
			panic!(
				"expected an enum value expression, got {:?}",
				attributes[0].value
			);
		};
		named.name.as_str().xpect_eq("ButtonVariant::Outlined");
	}

	#[crate::test]
	fn fragment_unquoted_bool_is_value_expr() {
		// an unquoted `true`/`false` resolves through the value grammar so an
		// embedded-markup component prop like `vertical_lines=true` reaches the
		// typed `bool` prop, instead of the string `"true"` failing prop validation
		// and dropping the whole template.
		let BsxFragmentToken::Open { attributes, .. } =
			&fragment("<Table vertical_lines=true>")[0]
		else {
			panic!("expected open");
		};
		attributes[0].value.clone().xpect_eq(AttrValue::Expr(
			ValueExpr::Literal(DataLiteral::Scalar(Value::Bool(true))),
		));
		// a bare word that is not a bool stays a plain string (an HTML URL/value).
		let BsxFragmentToken::Open { attributes, .. } =
			&fragment("<img loading=lazy>")[0]
		else {
			panic!("expected open");
		};
		attributes[0]
			.value
			.clone()
			.xpect_eq(AttrValue::Str("lazy".to_string()));
	}

	#[crate::test]
	fn fragment_braced_attr_uses_value_grammar() {
		let BsxFragmentToken::Open { attributes, .. } =
			&fragment("<button bx:click={count.increment}>")[0]
		else {
			panic!("expected open");
		};
		matches!(attributes[0].value, AttrValue::Expr(_)).xpect_true();
	}

	#[crate::test]
	fn fragment_spread() {
		let BsxFragmentToken::Open { attributes, .. } =
			&fragment("<el {MyComponent}>")[0]
		else {
			panic!("expected open");
		};
		matches!(attributes[0].value, AttrValue::Spread(_)).xpect_true();
	}

	#[crate::test]
	fn fragment_comment_and_doctype() {
		fragment("<!DOCTYPE html><!-- hi -->").xpect_eq(vec![
			BsxFragmentToken::Doctype("html"),
			BsxFragmentToken::Comment(" hi "),
		]);
	}

	#[crate::test]
	fn fragment_script_raw_text() {
		// `<` inside raw text is not a tag, so the script body stays one text run.
		BsxFragmentToken::parse_fragment(
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

	#[crate::test]
	fn fragment_expressions() {
		let config = BsxFragmentConfig {
			expressions: true,
			..Default::default()
		};
		BsxFragmentToken::parse_fragment("<p>hi {name}</p>", &config)
			.unwrap()
			.xpect_eq(vec![
				BsxFragmentToken::Open {
					tag: "p",
					attributes: vec![],
					self_closing: false,
				},
				BsxFragmentToken::Text("hi "),
				BsxFragmentToken::Expr("name"),
				BsxFragmentToken::Close { tag: "p" },
			]);
	}

	#[crate::test]
	fn fragment_raw_text_expressions() {
		let config = BsxFragmentConfig {
			raw_text_expressions: true,
			..Default::default()
		};
		BsxFragmentToken::parse_fragment("<style>a {{x}} b</style>", &config)
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

	/// The child nodes of a single root element parsed from `source`.
	fn children_of(source: &str) -> Vec<BsxNode> {
		let nodes =
			BsxNode::parse_document(source, &BsxParseConfig::bsx()).unwrap();
		let BsxNode::Element(element) = nodes.into_iter().next().unwrap()
		else {
			panic!("expected a root element");
		};
		element.children
	}

	#[crate::test]
	fn drops_formatting_whitespace_between_blocks() {
		// the indentation between block children is insignificant, like `rsx!` trims
		// and the browser collapses, so only the two elements survive.
		let children = children_of("<div>\n\t<h1>A</h1>\n\t<p>B</p>\n</div>");
		children.len().xpect_eq(2);
		matches!(children[0], BsxNode::Element(_)).xpect_true();
		matches!(children[1], BsxNode::Element(_)).xpect_true();
	}

	#[crate::test]
	fn keeps_significant_inline_whitespace() {
		// the whitespace separating prose from an inline link is significant, so it
		// stays (collapsing to a single space at render).
		let children =
			children_of("<p>say hi in the\n\t<a href=\"x\">Discord</a>.</p>");
		// "say hi in the…", <a>, "."
		children.len().xpect_eq(3);
		let BsxNode::Text(lead) = &children[0] else {
			panic!("expected leading text");
		};
		lead.ends_with(char::is_whitespace).xpect_true();
	}

	#[crate::test]
	fn preserves_pre_whitespace() {
		// `<pre>` content is whitespace-significant, so its newlines/indent survive.
		let children = children_of("<pre>\n  line one\n  line two\n</pre>");
		let BsxNode::Text(text) = &children[0] else {
			panic!("expected verbatim text");
		};
		text.clone()
			.xpect_eq("\n  line one\n  line two\n".to_string());
	}
}
