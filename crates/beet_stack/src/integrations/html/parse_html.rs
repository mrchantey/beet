//! HTML-to-ECS differ with positional diffing.
//!
//! [`HtmlDiffer`] implements [`Parser`] to reconcile an HTML string
//! directly against a Bevy entity hierarchy, spawning, updating, or
//! despawning entities as needed. Diffing is positional — only
//! entities whose content or structure actually changed are touched.
//!
//! # Usage
//!
//! ```
//! use beet_stack::prelude::*;
//! use beet_core::prelude::*;
//!
//! let mut world = World::new();
//! let root = world.spawn_empty().id();
//! HtmlDiffer::new("<h1>Hello</h1><p>world</p>")
//!     .diff(world.entity_mut(root))
//!     .unwrap();
//!
//! // root now has Heading1 and Paragraph children
//! let children = world.entity(root).get::<Children>().unwrap();
//! children.len().xpect_eq(2);
//! ```
//!
//! # Diffing
//!
//! Calling [`HtmlDiffer::diff`] on an entity that already has
//! children from a previous diff will reconcile positionally:
//!
//! - Matching node kinds at the same index are updated in place.
//! - Mismatched kinds cause the old subtree to be despawned and a new
//!   one spawned.
//! - Extra old children are despawned; extra new nodes are spawned.
use crate::prelude::*;
use beet_core::prelude::*;


// ---------------------------------------------------------------------------
// Intermediate representation
// ---------------------------------------------------------------------------

/// Intermediate tree node used during diffing.
#[derive(Debug, Clone, PartialEq)]
enum HtmlNode {
	/// An element with a tag and child nodes.
	Element {
		kind: HtmlElement,
		children: Vec<HtmlNode>,
	},
	/// A leaf text node.
	Text(String),
}

impl HtmlNode {
	fn element(kind: HtmlElement, children: Vec<HtmlNode>) -> Self {
		Self::Element { kind, children }
	}
	fn leaf(kind: HtmlElement) -> Self {
		Self::Element {
			kind,
			children: Vec::new(),
		}
	}
}

/// The semantic kind of an [`HtmlNode::Element`].
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
enum HtmlElement {
	// -- block --
	Paragraph,
	Heading(u8),
	BlockQuote,
	CodeBlock { language: Option<String> },
	OrderedList { start: u64 },
	UnorderedList,
	ListItem,
	Table,
	TableHead,
	TableRow,
	TableCell { header: bool },
	ThematicBreak,
	FootnoteDefinition { label: String },
	HtmlBlock(String),
	MathDisplay,
	Image { src: String, title: Option<String> },
	// -- inline wrappers --
	Strong,
	Emphasis,
	Strikethrough,
	Superscript,
	Subscript,
	Link { href: String, title: Option<String> },
	InlineCode,
	MathInline,
	HardBreak,
	SoftBreak,
	FootnoteRef { label: String },
	HtmlInline(String),
	TaskListCheck { checked: bool },
	Quote,
	Button,
}

impl HtmlElement {
	/// Returns true if two elements are the "same kind" for diffing
	/// purposes — same variant and same structural parameters.
	#[allow(dead_code)]
	fn same_kind(&self, other: &Self) -> bool {
		use HtmlElement::*;
		match (self, other) {
			(Paragraph, Paragraph)
			| (BlockQuote, BlockQuote)
			| (UnorderedList, UnorderedList)
			| (ListItem, ListItem)
			| (TableHead, TableHead)
			| (TableRow, TableRow)
			| (Table, Table)
			| (ThematicBreak, ThematicBreak)
			| (MathDisplay, MathDisplay)
			| (Strong, Strong)
			| (Emphasis, Emphasis)
			| (Strikethrough, Strikethrough)
			| (Superscript, Superscript)
			| (Subscript, Subscript)
			| (InlineCode, InlineCode)
			| (MathInline, MathInline)
			| (HardBreak, HardBreak)
			| (SoftBreak, SoftBreak)
			| (Quote, Quote)
			| (Button, Button) => true,
			(Heading(level_a), Heading(level_b)) => level_a == level_b,
			(
				OrderedList { start: start_a },
				OrderedList { start: start_b },
			) => start_a == start_b,
			(
				CodeBlock { language: lang_a },
				CodeBlock { language: lang_b },
			) => lang_a == lang_b,
			(
				TableCell { header: header_a },
				TableCell { header: header_b },
			) => header_a == header_b,
			(
				FootnoteDefinition { label: label_a },
				FootnoteDefinition { label: label_b },
			) => label_a == label_b,
			(Link { .. }, Link { .. }) => true,
			(Image { .. }, Image { .. }) => true,
			(
				FootnoteRef { label: label_a },
				FootnoteRef { label: label_b },
			) => label_a == label_b,
			(
				TaskListCheck { checked: checked_a },
				TaskListCheck { checked: checked_b },
			) => checked_a == checked_b,
			_ => false,
		}
	}
}


// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

/// Reconciles an HTML string against an entity's children.
///
/// Implements [`Parser`] so it can be used with the generic parsing
/// interface. Internally uses a lightweight state-machine tokenizer
/// to parse the HTML and positionally diffs the result against the
/// existing entity hierarchy.
pub struct HtmlDiffer<'a> {
	html: &'a str,
}

impl<'a> HtmlDiffer<'a> {
	/// Create a differ for the given HTML text.
	pub fn new(html: &'a str) -> Self { Self { html } }

	/// Parse the HTML text into an intermediate node tree.
	fn parse(&self) -> Vec<HtmlNode> {
		let mut builder = TreeBuilder::default();
		let mut tokenizer = Tokenizer::new(self.html);
		while let Some(token) = tokenizer.next_token() {
			builder.handle_token(token);
		}
		builder.finish()
	}
}

impl Parser for HtmlDiffer<'_> {
	fn diff(&mut self, entity: EntityWorldMut) -> Result {
		let nodes = self.parse();
		let entity_id = entity.id();
		let world = entity.into_world_mut();
		diff_children(world, entity_id, &nodes);
		Ok(())
	}
}


// ---------------------------------------------------------------------------
// Tokenizer — converts HTML string into tokens
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Token {
	/// `<tag attr="val">` or `<tag>`
	OpenTag {
		tag: String,
		attrs: Vec<(String, String)>,
		self_closing: bool,
	},
	/// `</tag>`
	CloseTag { tag: String },
	/// Raw text content between tags.
	Text(String),
}

struct Tokenizer<'a> {
	input: &'a str,
	pos: usize,
}

impl<'a> Tokenizer<'a> {
	fn new(input: &'a str) -> Self { Self { input, pos: 0 } }

	fn remaining(&self) -> &'a str { &self.input[self.pos..] }

	fn next_token(&mut self) -> Option<Token> {
		if self.pos >= self.input.len() {
			return None;
		}

		if self.remaining().starts_with('<') {
			self.read_tag()
		} else {
			self.read_text()
		}
	}

	fn read_text(&mut self) -> Option<Token> {
		let start = self.pos;
		while self.pos < self.input.len()
			&& self.input.as_bytes()[self.pos] != b'<'
		{
			self.pos += 1;
		}
		if self.pos == start {
			return None;
		}
		let raw = &self.input[start..self.pos];
		let decoded = decode_html_entities(raw);
		Some(Token::Text(decoded))
	}

	fn read_tag(&mut self) -> Option<Token> {
		// Skip '<'
		self.pos += 1;
		self.skip_whitespace();

		if self.pos >= self.input.len() {
			return None;
		}

		// Check for closing tag
		let is_close = self.remaining().starts_with('/');
		if is_close {
			self.pos += 1;
			self.skip_whitespace();
		}

		// Read tag name
		let tag = self.read_tag_name();
		if tag.is_empty() {
			// Skip to '>' and treat as text
			while self.pos < self.input.len()
				&& self.input.as_bytes()[self.pos] != b'>'
			{
				self.pos += 1;
			}
			if self.pos < self.input.len() {
				self.pos += 1;
			}
			return self.next_token();
		}

		if is_close {
			// Skip to '>'
			while self.pos < self.input.len()
				&& self.input.as_bytes()[self.pos] != b'>'
			{
				self.pos += 1;
			}
			if self.pos < self.input.len() {
				self.pos += 1;
			}
			return Some(Token::CloseTag {
				tag: tag.to_ascii_lowercase(),
			});
		}

		// Read attributes
		let mut attrs = Vec::new();
		loop {
			self.skip_whitespace();
			if self.pos >= self.input.len() {
				break;
			}
			let ch = self.input.as_bytes()[self.pos];
			if ch == b'>' || ch == b'/' {
				break;
			}
			if let Some((key, val)) = self.read_attribute() {
				attrs.push((key, val));
			} else {
				// Skip unrecognized character
				self.pos += 1;
			}
		}

		// Check for self-closing '/>'
		let mut self_closing = false;
		if self.pos < self.input.len()
			&& self.input.as_bytes()[self.pos] == b'/'
		{
			self_closing = true;
			self.pos += 1;
		}

		// Skip '>'
		if self.pos < self.input.len()
			&& self.input.as_bytes()[self.pos] == b'>'
		{
			self.pos += 1;
		}

		let lower_tag = tag.to_ascii_lowercase();

		// Void elements are always self-closing
		if is_void_element(&lower_tag) {
			self_closing = true;
		}

		Some(Token::OpenTag {
			tag: lower_tag,
			attrs,
			self_closing,
		})
	}

	fn read_tag_name(&mut self) -> String {
		let start = self.pos;
		while self.pos < self.input.len() {
			let ch = self.input.as_bytes()[self.pos];
			if ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'_' {
				self.pos += 1;
			} else {
				break;
			}
		}
		self.input[start..self.pos].to_string()
	}

	fn read_attribute(&mut self) -> Option<(String, String)> {
		let key = self.read_attr_name();
		if key.is_empty() {
			return None;
		}

		self.skip_whitespace();

		// Check for '='
		if self.pos < self.input.len()
			&& self.input.as_bytes()[self.pos] == b'='
		{
			self.pos += 1;
			self.skip_whitespace();
			let val = self.read_attr_value();
			Some((key.to_ascii_lowercase(), val))
		} else {
			// Boolean attribute
			Some((key.to_ascii_lowercase(), String::new()))
		}
	}

	fn read_attr_name(&mut self) -> String {
		let start = self.pos;
		while self.pos < self.input.len() {
			let ch = self.input.as_bytes()[self.pos];
			if ch == b'='
				|| ch == b'>'
				|| ch == b'/'
				|| ch.is_ascii_whitespace()
			{
				break;
			}
			self.pos += 1;
		}
		self.input[start..self.pos].to_string()
	}

	fn read_attr_value(&mut self) -> String {
		if self.pos >= self.input.len() {
			return String::new();
		}

		let ch = self.input.as_bytes()[self.pos];
		if ch == b'"' || ch == b'\'' {
			let quote = ch;
			self.pos += 1;
			let start = self.pos;
			while self.pos < self.input.len()
				&& self.input.as_bytes()[self.pos] != quote
			{
				self.pos += 1;
			}
			let val = &self.input[start..self.pos];
			if self.pos < self.input.len() {
				self.pos += 1; // skip closing quote
			}
			decode_html_entities(val)
		} else {
			// Unquoted attribute value
			let start = self.pos;
			while self.pos < self.input.len() {
				let ch = self.input.as_bytes()[self.pos];
				if ch.is_ascii_whitespace() || ch == b'>' || ch == b'/' {
					break;
				}
				self.pos += 1;
			}
			decode_html_entities(&self.input[start..self.pos])
		}
	}

	fn skip_whitespace(&mut self) {
		while self.pos < self.input.len()
			&& self.input.as_bytes()[self.pos].is_ascii_whitespace()
		{
			self.pos += 1;
		}
	}
}

fn is_void_element(tag: &str) -> bool {
	matches!(
		tag,
		"area"
			| "base" | "br"
			| "col" | "embed"
			| "hr" | "img"
			| "input" | "link"
			| "meta" | "param"
			| "source"
			| "track" | "wbr"
	)
}

/// Decode common HTML entities.
fn decode_html_entities(text: &str) -> String {
	if !text.contains('&') {
		return text.to_string();
	}
	let mut result = String::with_capacity(text.len());
	let mut chars = text.chars().peekable();
	while let Some(ch) = chars.next() {
		if ch == '&' {
			let mut entity = String::new();
			let mut found_semicolon = false;
			for next in chars.by_ref() {
				if next == ';' {
					found_semicolon = true;
					break;
				}
				entity.push(next);
				if entity.len() > 10 {
					break;
				}
			}
			if found_semicolon {
				match entity.as_str() {
					"amp" => result.push('&'),
					"lt" => result.push('<'),
					"gt" => result.push('>'),
					"quot" => result.push('"'),
					"apos" => result.push('\''),
					"nbsp" => result.push('\u{00A0}'),
					other
						if other.starts_with("#x")
							|| other.starts_with("#X") =>
					{
						if let Ok(code) = u32::from_str_radix(&other[2..], 16) {
							if let Some(decoded) = char::from_u32(code) {
								result.push(decoded);
							} else {
								result.push('&');
								result.push_str(other);
								result.push(';');
							}
						} else {
							result.push('&');
							result.push_str(other);
							result.push(';');
						}
					}
					other if other.starts_with('#') => {
						if let Ok(code) = other[1..].parse::<u32>() {
							if let Some(decoded) = char::from_u32(code) {
								result.push(decoded);
							} else {
								result.push('&');
								result.push_str(other);
								result.push(';');
							}
						} else {
							result.push('&');
							result.push_str(other);
							result.push(';');
						}
					}
					_ => {
						result.push('&');
						result.push_str(&entity);
						result.push(';');
					}
				}
			} else {
				result.push('&');
				result.push_str(&entity);
			}
		} else {
			result.push(ch);
		}
	}
	result
}


// ---------------------------------------------------------------------------
// Tree builder — converts tokens into HtmlNode tree
// ---------------------------------------------------------------------------

struct TreeBuilder {
	/// Stack of (element_kind, children_so_far).
	/// The bottom entry is a synthetic root whose children become the
	/// final output.
	stack: Vec<(Option<HtmlElement>, Vec<HtmlNode>)>,
	/// Tracks whether we are inside a `<pre>` element.
	in_pre: bool,
}

impl Default for TreeBuilder {
	fn default() -> Self {
		Self {
			stack: vec![(None, Vec::new())],
			in_pre: false,
		}
	}
}

impl TreeBuilder {
	fn push(&mut self, kind: HtmlElement) {
		self.stack.push((Some(kind), Vec::new()));
	}

	fn pop(&mut self) -> Option<HtmlElement> {
		if self.stack.len() <= 1 {
			return None;
		}
		if let Some((kind, children)) = self.stack.pop() {
			let node = match kind {
				Some(ref kind_val) => {
					HtmlNode::element(kind_val.clone(), children)
				}
				None => return None,
			};
			if let Some(parent) = self.stack.last_mut() {
				parent.1.push(node);
			}
			kind
		} else {
			None
		}
	}

	fn push_leaf(&mut self, node: HtmlNode) {
		if let Some(parent) = self.stack.last_mut() {
			parent.1.push(node);
		}
	}

	fn finish(mut self) -> Vec<HtmlNode> {
		// Drain any unclosed elements
		while self.stack.len() > 1 {
			self.pop();
		}
		self.stack
			.pop()
			.map(|(_, children)| children)
			.unwrap_or_default()
	}

	/// Get an attribute value from attrs list.
	fn get_attr<'a>(
		attrs: &'a [(String, String)],
		key: &str,
	) -> Option<&'a str> {
		attrs
			.iter()
			.find(|(k, _)| k == key)
			.map(|(_, val)| val.as_str())
	}

	/// Check if an attribute is present (boolean attribute).
	fn has_attr(attrs: &[(String, String)], key: &str) -> bool {
		attrs.iter().any(|(k, _)| k == key)
	}

	fn handle_token(&mut self, token: Token) {
		match token {
			Token::Text(text) => {
				if !text.is_empty() {
					// Inside <pre><code>, preserve text as-is
					// Outside, collapse whitespace-only runs between
					// block elements (but keep meaningful text)
					if self.in_pre || !text.trim().is_empty() {
						self.push_leaf(HtmlNode::Text(text));
					}
				}
			}
			Token::OpenTag {
				tag,
				attrs,
				self_closing,
			} => {
				self.handle_open_tag(&tag, &attrs, self_closing);
			}
			Token::CloseTag { tag } => {
				self.handle_close_tag(&tag);
			}
		}
	}

	fn handle_open_tag(
		&mut self,
		tag: &str,
		attrs: &[(String, String)],
		self_closing: bool,
	) {
		match tag {
			"p" => {
				self.push(HtmlElement::Paragraph);
			}
			"h1" => self.push(HtmlElement::Heading(1)),
			"h2" => self.push(HtmlElement::Heading(2)),
			"h3" => self.push(HtmlElement::Heading(3)),
			"h4" => self.push(HtmlElement::Heading(4)),
			"h5" => self.push(HtmlElement::Heading(5)),
			"h6" => self.push(HtmlElement::Heading(6)),
			"blockquote" => self.push(HtmlElement::BlockQuote),
			"pre" => {
				self.in_pre = true;
				// The next token should be <code>, which we handle
				// specially. For now push a marker. We'll merge
				// pre+code into CodeBlock when we see the <code> child.
				// Actually, handle it simply: pre starts a code block
				// context, and if code follows we extract the language.
				// We don't push anything for <pre> itself; the <code>
				// inside will create the CodeBlock.
				// HOWEVER, if there's no <code> inside, we still need
				// to handle it. Let's push a CodeBlock with no language.
				self.push(HtmlElement::CodeBlock { language: None });
			}
			"code" => {
				if self.in_pre {
					// We're inside a <pre> — update the CodeBlock's
					// language from the class attribute.
					if let Some(class) = Self::get_attr(attrs, "class") {
						if let Some(lang) = class.strip_prefix("language-") {
							// Update the top of the stack
							if let Some((
								Some(HtmlElement::CodeBlock { language }),
								_,
							)) = self.stack.last_mut()
							{
								*language = Some(lang.to_string());
							}
						}
					}
					// Don't push a new element — the <pre> already
					// created the CodeBlock. We just skip the <code>
					// open tag.
				} else {
					// Inline code
					self.push(HtmlElement::InlineCode);
				}
			}
			"ul" => self.push(HtmlElement::UnorderedList),
			"ol" => {
				let start = Self::get_attr(attrs, "start")
					.and_then(|val| val.parse::<u64>().ok())
					.unwrap_or(1);
				self.push(HtmlElement::OrderedList { start });
			}
			"li" => self.push(HtmlElement::ListItem),
			"table" => self.push(HtmlElement::Table),
			"thead" => self.push(HtmlElement::TableHead),
			"tbody" => {
				// tbody is implicit in our model — just recurse children
				// Don't push anything; children (tr) go to the table.
			}
			"tr" => self.push(HtmlElement::TableRow),
			"th" => self.push(HtmlElement::TableCell { header: true }),
			"td" => self.push(HtmlElement::TableCell { header: false }),
			"hr" => {
				self.push_leaf(HtmlNode::leaf(HtmlElement::ThematicBreak));
			}
			"br" => {
				self.push_leaf(HtmlNode::leaf(HtmlElement::HardBreak));
			}
			"img" => {
				let src =
					Self::get_attr(attrs, "src").unwrap_or("").to_string();
				let title = Self::get_attr(attrs, "title")
					.filter(|title| !title.is_empty())
					.map(|title| title.to_string());
				self.push_leaf(HtmlNode::leaf(HtmlElement::Image {
					src,
					title,
				}));
			}
			"input" => {
				// Task list checkbox
				let input_type = Self::get_attr(attrs, "type").unwrap_or("");
				if input_type == "checkbox" {
					let checked = Self::has_attr(attrs, "checked");
					self.push_leaf(HtmlNode::leaf(
						HtmlElement::TaskListCheck { checked },
					));
				}
			}
			"strong" | "b" => self.push(HtmlElement::Strong),
			"em" | "i" => self.push(HtmlElement::Emphasis),
			"del" | "s" | "strike" => self.push(HtmlElement::Strikethrough),
			"sup" => {
				// Check if this is a footnote ref
				if let Some(id) = Self::get_attr(attrs, "id") {
					if let Some(label) = id.strip_prefix("fnref-") {
						self.push_leaf(HtmlNode::leaf(
							HtmlElement::FootnoteRef {
								label: label.to_string(),
							},
						));
						// Skip the inner <a> content — it will be
						// consumed by close tags but we've already
						// emitted the leaf.
						return;
					}
				}
				self.push(HtmlElement::Superscript);
			}
			"sub" => self.push(HtmlElement::Subscript),
			"q" => self.push(HtmlElement::Quote),
			"a" => {
				let href =
					Self::get_attr(attrs, "href").unwrap_or("").to_string();

				// Check if this is a footnote back-link inside a
				// footnote definition. If so, skip it.
				if href.starts_with("#fnref-") {
					return;
				}

				let title = Self::get_attr(attrs, "title")
					.filter(|title| !title.is_empty())
					.map(|title| title.to_string());
				self.push(HtmlElement::Link { href, title });
			}
			"button" => self.push(HtmlElement::Button),
			"div" => {
				// Check for special div types
				if let Some(class) = Self::get_attr(attrs, "class") {
					if class.contains("footnote") {
						if let Some(id) = Self::get_attr(attrs, "id") {
							if let Some(label) = id.strip_prefix("fn-") {
								self.push(HtmlElement::FootnoteDefinition {
									label: label.to_string(),
								});
								return;
							}
						}
					}
					if class.contains("math-display") {
						self.push(HtmlElement::MathDisplay);
						return;
					}
				}
				// Generic div — treat as unknown, just recurse
				// We don't push anything to keep the tree clean
			}
			"span" => {
				if let Some(class) = Self::get_attr(attrs, "class") {
					if class.contains("math-inline") {
						self.push(HtmlElement::MathInline);
						return;
					}
				}
				// Generic span — ignore the wrapper
			}
			_ => {
				// Unknown tag — if self-closing, ignore. Otherwise
				// we don't track it but content inside will still be
				// collected by the parent.
				if !self_closing {
					// We could push a generic wrapper but for now
					// just ignore the tag boundaries.
				}
			}
		}

		// For self-closing tags that we pushed as containers, pop them
		if self_closing && !matches!(tag, "hr" | "br" | "img" | "input") {
			// Some tags were pushed as containers but are self-closing
			self.pop();
		}
	}

	fn handle_close_tag(&mut self, tag: &str) {
		match tag {
			"pre" => {
				self.in_pre = false;
				self.pop();
			}
			"code" if self.in_pre => {
				// Closing </code> inside <pre> — we didn't push a
				// separate element for it, so don't pop.
			}
			"code" => {
				self.pop();
			}
			"tbody" => {
				// We didn't push tbody, so don't pop
			}
			"div" | "span" => {
				// Only pop if we actually pushed something for this
				// tag (footnote def, math display, math inline).
				if let Some((Some(kind), _)) = self.stack.last() {
					match kind {
						HtmlElement::FootnoteDefinition { .. }
						| HtmlElement::MathDisplay
						| HtmlElement::MathInline => {
							self.pop();
						}
						_ => {}
					}
				}
			}
			// Void elements — no close tag expected but handle gracefully
			"hr" | "br" | "img" | "input" => {}
			// All other tags — pop
			_ => {
				self.pop();
			}
		}
	}
}


// ---------------------------------------------------------------------------
// ECS diffing
// ---------------------------------------------------------------------------

/// Diff/apply a list of [`HtmlNode`] as the children of `parent`.
fn diff_children(world: &mut World, parent: Entity, nodes: &[HtmlNode]) {
	let existing: Vec<Entity> = world
		.entity(parent)
		.get::<Children>()
		.map(|children| children.iter().collect())
		.unwrap_or_default();

	let mut keep_count = 0usize;

	for (idx, node) in nodes.iter().enumerate() {
		if idx < existing.len() {
			let child = existing[idx];
			if can_reuse(world, child, node) {
				update_in_place(world, child, node);
				keep_count += 1;
			} else {
				world.despawn(child);
				let new_child = spawn_node(world, node);
				world.entity_mut(new_child).insert(ChildOf(parent));
				keep_count += 1;
			}
		} else {
			let new_child = spawn_node(world, node);
			world.entity_mut(new_child).insert(ChildOf(parent));
		}
	}

	// Despawn extra old children
	for idx in (keep_count..existing.len()).rev() {
		world.despawn(existing[idx]);
	}
}

/// Check whether an existing entity can be reused for the given node.
fn can_reuse(world: &World, entity: Entity, node: &HtmlNode) -> bool {
	match node {
		HtmlNode::Text(_) => world.entity(entity).contains::<TextNode>(),
		HtmlNode::Element { kind, .. } => {
			entity_matches_kind(world, entity, kind)
		}
	}
}

/// Check whether an existing entity matches a specific [`HtmlElement`] kind.
fn entity_matches_kind(
	world: &World,
	entity: Entity,
	kind: &HtmlElement,
) -> bool {
	let entity_ref = world.entity(entity);
	match kind {
		HtmlElement::Paragraph => entity_ref.contains::<Paragraph>(),
		HtmlElement::Heading(level) => entity_ref
			.get::<Heading>()
			.is_some_and(|heading| heading.level() == *level),
		HtmlElement::BlockQuote => entity_ref.contains::<BlockQuote>(),
		HtmlElement::CodeBlock { language } => entity_ref
			.get::<CodeBlock>()
			.is_some_and(|cb| cb.language.as_deref() == language.as_deref()),
		HtmlElement::OrderedList { start } => entity_ref
			.get::<ListMarker>()
			.is_some_and(|lm| lm.ordered && lm.start == Some(*start)),
		HtmlElement::UnorderedList => {
			entity_ref.get::<ListMarker>().is_some_and(|lm| !lm.ordered)
		}
		HtmlElement::ListItem => entity_ref.contains::<ListItem>(),
		HtmlElement::Table => entity_ref.contains::<Table>(),
		HtmlElement::TableHead => entity_ref.contains::<TableHead>(),
		HtmlElement::TableRow => entity_ref.contains::<TableRow>(),
		HtmlElement::TableCell { header } => entity_ref
			.get::<TableCell>()
			.is_some_and(|tc| tc.header == *header),
		HtmlElement::ThematicBreak => entity_ref.contains::<ThematicBreak>(),
		HtmlElement::FootnoteDefinition { label } => entity_ref
			.get::<FootnoteDefinition>()
			.is_some_and(|fd| fd.label == *label),
		HtmlElement::HtmlBlock(_) => entity_ref.contains::<HtmlBlock>(),
		HtmlElement::MathDisplay => entity_ref.contains::<MathDisplay>(),
		HtmlElement::Image { .. } => entity_ref.contains::<Image>(),
		HtmlElement::Strong => entity_ref.contains::<Important>(),
		HtmlElement::Emphasis => entity_ref.contains::<Emphasize>(),
		HtmlElement::Strikethrough => entity_ref.contains::<Strikethrough>(),
		HtmlElement::Superscript => entity_ref.contains::<Superscript>(),
		HtmlElement::Subscript => entity_ref.contains::<Subscript>(),
		HtmlElement::Link { .. } => entity_ref.contains::<Link>(),
		HtmlElement::InlineCode => entity_ref.contains::<Code>(),
		HtmlElement::MathInline => entity_ref.contains::<MathInline>(),
		HtmlElement::HardBreak => entity_ref.contains::<HardBreak>(),
		HtmlElement::SoftBreak => entity_ref.contains::<SoftBreak>(),
		HtmlElement::FootnoteRef { label } => entity_ref
			.get::<FootnoteRef>()
			.is_some_and(|fr| fr.label == *label),
		HtmlElement::HtmlInline(_) => entity_ref.contains::<HtmlInline>(),
		HtmlElement::TaskListCheck { checked } => entity_ref
			.get::<TaskListCheck>()
			.is_some_and(|tc| tc.checked == *checked),
		HtmlElement::Quote => entity_ref.contains::<Quote>(),
		HtmlElement::Button => entity_ref.contains::<Button>(),
	}
}

/// Update an existing entity in place to match the new node.
fn update_in_place(world: &mut World, entity: Entity, node: &HtmlNode) {
	match node {
		HtmlNode::Text(text) => {
			if let Some(mut existing) =
				world.entity_mut(entity).get_mut::<TextNode>()
			{
				if existing.0 != *text {
					existing.0 = text.clone();
				}
			}
		}
		HtmlNode::Element { kind, children } => {
			update_element_data(world, entity, kind);
			diff_children(world, entity, children);
		}
	}
}

/// Update the mutable data fields of an element component.
fn update_element_data(world: &mut World, entity: Entity, kind: &HtmlElement) {
	match kind {
		HtmlElement::Link { href, title } => {
			if let Some(mut link) = world.entity_mut(entity).get_mut::<Link>() {
				if link.href != *href {
					link.href = href.clone();
				}
				if link.title != *title {
					link.title = title.clone();
				}
			}
		}
		HtmlElement::Image { src, title } => {
			if let Some(mut image) = world.entity_mut(entity).get_mut::<Image>()
			{
				if image.src != *src {
					image.src = src.clone();
				}
				if image.title != *title {
					image.title = title.clone();
				}
			}
		}
		HtmlElement::HtmlBlock(content) => {
			if let Some(mut block) =
				world.entity_mut(entity).get_mut::<HtmlBlock>()
			{
				if block.0 != *content {
					block.0 = content.clone();
				}
			}
		}
		HtmlElement::HtmlInline(content) => {
			if let Some(mut inline) =
				world.entity_mut(entity).get_mut::<HtmlInline>()
			{
				if inline.0 != *content {
					inline.0 = content.clone();
				}
			}
		}
		_ => {}
	}
}


/// Spawn a new entity subtree for a node and return the root entity id.
fn spawn_node(world: &mut World, node: &HtmlNode) -> Entity {
	match node {
		HtmlNode::Text(text) => world.spawn(TextNode::new(text.as_str())).id(),
		HtmlNode::Element { kind, children } => {
			let entity = spawn_element(world, kind);
			for child_node in children {
				let child = spawn_node(world, child_node);
				world.entity_mut(child).insert(ChildOf(entity));
			}
			entity
		}
	}
}

/// Spawn a single element entity (no children yet) for the given kind.
fn spawn_element(world: &mut World, kind: &HtmlElement) -> Entity {
	match kind {
		HtmlElement::Paragraph => world.spawn(Paragraph).id(),
		HtmlElement::Heading(1) => world.spawn(Heading1).id(),
		HtmlElement::Heading(2) => world.spawn(Heading2).id(),
		HtmlElement::Heading(3) => world.spawn(Heading3).id(),
		HtmlElement::Heading(4) => world.spawn(Heading4).id(),
		HtmlElement::Heading(5) => world.spawn(Heading5).id(),
		HtmlElement::Heading(6) => world.spawn(Heading6).id(),
		HtmlElement::Heading(_) => world.spawn(Heading1).id(),
		HtmlElement::BlockQuote => world.spawn(BlockQuote).id(),
		HtmlElement::CodeBlock { language } => world
			.spawn(match language {
				Some(lang) => CodeBlock::with_language(lang),
				None => CodeBlock::plain(),
			})
			.id(),
		HtmlElement::OrderedList { start } => {
			world.spawn(ListMarker::ordered(*start)).id()
		}
		HtmlElement::UnorderedList => world.spawn(ListMarker::unordered()).id(),
		HtmlElement::ListItem => world.spawn(ListItem).id(),
		HtmlElement::Table => world
			.spawn(Table {
				alignments: Vec::new(),
			})
			.id(),
		HtmlElement::TableHead => world.spawn(TableHead).id(),
		HtmlElement::TableRow => world.spawn(TableRow).id(),
		HtmlElement::TableCell { header } => world
			.spawn(TableCell {
				header: *header,
				alignment: TextAlignment::None,
			})
			.id(),
		HtmlElement::ThematicBreak => world.spawn(ThematicBreak).id(),
		HtmlElement::FootnoteDefinition { label } => world
			.spawn(FootnoteDefinition {
				label: label.clone(),
			})
			.id(),
		HtmlElement::HtmlBlock(content) => {
			world.spawn(HtmlBlock(content.clone())).id()
		}
		HtmlElement::MathDisplay => world.spawn(MathDisplay).id(),
		HtmlElement::Image { src, title } => world
			.spawn({
				let image = Image::new(src);
				match title {
					Some(title) => image.with_title(title),
					None => image,
				}
			})
			.id(),
		HtmlElement::Strong => world.spawn(Important).id(),
		HtmlElement::Emphasis => world.spawn(Emphasize).id(),
		HtmlElement::Strikethrough => world.spawn(Strikethrough).id(),
		HtmlElement::Superscript => world.spawn(Superscript).id(),
		HtmlElement::Subscript => world.spawn(Subscript).id(),
		HtmlElement::Link { href, title } => {
			let link = Link::new(href);
			let link = match title {
				Some(title) => link.with_title(title),
				None => link,
			};
			world.spawn(link).id()
		}
		HtmlElement::InlineCode => world.spawn(Code).id(),
		HtmlElement::MathInline => world.spawn(MathInline).id(),
		HtmlElement::HardBreak => world.spawn(HardBreak).id(),
		HtmlElement::SoftBreak => world.spawn(SoftBreak).id(),
		HtmlElement::FootnoteRef { label } => world
			.spawn(FootnoteRef {
				label: label.clone(),
			})
			.id(),
		HtmlElement::HtmlInline(content) => {
			world.spawn(HtmlInline(content.clone())).id()
		}
		HtmlElement::TaskListCheck { checked } => world
			.spawn(if *checked {
				TaskListCheck::checked()
			} else {
				TaskListCheck::unchecked()
			})
			.id(),
		HtmlElement::Quote => world.spawn(Quote).id(),
		HtmlElement::Button => world.spawn(Button::default()).id(),
	}
}


// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod test {
	use super::*;

	fn parse(html: &str) -> Vec<HtmlNode> { HtmlDiffer::new(html).parse() }

	fn render(world: &mut World, entity: Entity) -> String {
		world
			.run_system_once_with(
				|In(entity): In<Entity>, walker: CardWalker| {
					let mut renderer = HtmlRenderer::new();
					walker.walk_card(&mut renderer, entity);
					renderer.finish()
				},
				entity,
			)
			.unwrap()
	}

	fn child_entities(world: &World, entity: Entity) -> Vec<Entity> {
		world
			.entity(entity)
			.get::<Children>()
			.map(|children| children.iter().collect())
			.unwrap_or_default()
	}

	fn collect_text(world: &World, entity: Entity) -> String {
		let mut result = String::new();
		if let Some(text) = world.entity(entity).get::<TextNode>() {
			result.push_str(text.as_str());
		}
		for child in child_entities(world, entity) {
			result.push_str(&collect_text(world, child));
		}
		result
	}

	// -- Tokenizer --

	#[test]
	fn tokenize_simple_tag() {
		let mut tokenizer = Tokenizer::new("<p>hello</p>");
		let tokens: Vec<_> =
			std::iter::from_fn(|| tokenizer.next_token()).collect();
		tokens.len().xpect_eq(3);
	}

	#[test]
	fn tokenize_self_closing() {
		let mut tokenizer = Tokenizer::new("<hr />");
		let token = tokenizer.next_token().unwrap();
		matches!(token, Token::OpenTag {
			self_closing: true,
			..
		})
		.xpect_true();
	}

	#[test]
	fn tokenize_attributes() {
		let mut tokenizer =
			Tokenizer::new("<a href=\"https://example.com\">link</a>");
		let token = tokenizer.next_token().unwrap();
		if let Token::OpenTag { attrs, .. } = token {
			attrs.len().xpect_eq(1);
			attrs[0].0.as_str().xpect_eq("href");
			attrs[0].1.as_str().xpect_eq("https://example.com");
		} else {
			panic!("expected OpenTag");
		}
	}

	#[test]
	fn decode_entities() {
		decode_html_entities("&amp;").xpect_eq("&");
		decode_html_entities("&lt;").xpect_eq("<");
		decode_html_entities("&gt;").xpect_eq(">");
		decode_html_entities("&quot;").xpect_eq("\"");
		decode_html_entities("&#x27;").xpect_eq("'");
		decode_html_entities("no entities here").xpect_eq("no entities here");
		decode_html_entities("A &amp; B").xpect_eq("A & B");
	}

	// -- Parse --

	#[test]
	fn parse_paragraph() {
		let nodes = parse("<p>hello world</p>");
		nodes.len().xpect_eq(1);
		matches!(&nodes[0], HtmlNode::Element {
			kind: HtmlElement::Paragraph,
			..
		})
		.xpect_true();
	}

	#[test]
	fn parse_heading() {
		let nodes = parse("<h1>Title</h1>");
		nodes.len().xpect_eq(1);
		matches!(&nodes[0], HtmlNode::Element {
			kind: HtmlElement::Heading(1),
			..
		})
		.xpect_true();
	}

	#[test]
	fn parse_multiple_headings() {
		let nodes = parse("<h1>One</h1><h2>Two</h2><h3>Three</h3>");
		nodes.len().xpect_eq(3);
		matches!(&nodes[0], HtmlNode::Element {
			kind: HtmlElement::Heading(1),
			..
		})
		.xpect_true();
		matches!(&nodes[1], HtmlNode::Element {
			kind: HtmlElement::Heading(2),
			..
		})
		.xpect_true();
		matches!(&nodes[2], HtmlNode::Element {
			kind: HtmlElement::Heading(3),
			..
		})
		.xpect_true();
	}

	#[test]
	fn parse_emphasis_and_strong() {
		let nodes =
			parse("<p>hello <strong>bold</strong> and <em>italic</em></p>");
		nodes.len().xpect_eq(1);
		if let HtmlNode::Element { children, .. } = &nodes[0] {
			// "hello " + <strong> + " and " + <em>
			children.len().xpect_eq(4);
			matches!(&children[0], HtmlNode::Text(text) if text == "hello ")
				.xpect_true();
			matches!(&children[1], HtmlNode::Element {
				kind: HtmlElement::Strong,
				..
			})
			.xpect_true();
		} else {
			panic!("expected element");
		}
	}

	#[test]
	fn parse_code_block() {
		let nodes = parse(
			"<pre><code class=\"language-rust\">fn main() {}</code></pre>",
		);
		nodes.len().xpect_eq(1);
		if let HtmlNode::Element {
			kind: HtmlElement::CodeBlock { language },
			children,
		} = &nodes[0]
		{
			language.as_deref().xpect_eq(Some("rust"));
			children.len().xpect_eq(1);
			matches!(&children[0], HtmlNode::Text(text) if text == "fn main() {}")
				.xpect_true();
		} else {
			panic!("expected CodeBlock, got {:?}", &nodes[0]);
		}
	}

	#[test]
	fn parse_unordered_list() {
		let nodes = parse("<ul><li>one</li><li>two</li><li>three</li></ul>");
		nodes.len().xpect_eq(1);
		if let HtmlNode::Element {
			kind: HtmlElement::UnorderedList,
			children,
		} = &nodes[0]
		{
			children.len().xpect_eq(3);
		} else {
			panic!("expected UnorderedList");
		}
	}

	#[test]
	fn parse_link() {
		let nodes = parse(
			"<a href=\"https://example.com\" title=\"Example\">click</a>",
		);
		nodes.len().xpect_eq(1);
		if let HtmlNode::Element {
			kind: HtmlElement::Link { href, title },
			children,
		} = &nodes[0]
		{
			href.as_str().xpect_eq("https://example.com");
			title.as_deref().xpect_eq(Some("Example"));
			children.len().xpect_eq(1);
		} else {
			panic!("expected Link");
		}
	}

	#[test]
	fn parse_blockquote() {
		let nodes = parse("<blockquote><p>quoted</p></blockquote>");
		nodes.len().xpect_eq(1);
		matches!(&nodes[0], HtmlNode::Element {
			kind: HtmlElement::BlockQuote,
			..
		})
		.xpect_true();
	}

	#[test]
	fn parse_image() {
		let nodes = parse("<img src=\"test.png\" title=\"A test\" />");
		nodes.len().xpect_eq(1);
		if let HtmlNode::Element {
			kind: HtmlElement::Image { src, title },
			..
		} = &nodes[0]
		{
			src.as_str().xpect_eq("test.png");
			title.as_deref().xpect_eq(Some("A test"));
		} else {
			panic!("expected Image");
		}
	}

	// -- Diff (spawn) --

	#[test]
	fn diff_spawns_paragraph() {
		let mut world = World::new();
		let root = world.spawn_empty().id();
		HtmlDiffer::new("<p>hello world</p>")
			.diff(world.entity_mut(root))
			.unwrap();

		let children = child_entities(&world, root);
		children.len().xpect_eq(1);
		world
			.entity(children[0])
			.contains::<Paragraph>()
			.xpect_true();
	}

	#[test]
	fn diff_spawns_heading() {
		let mut world = World::new();
		let root = world.spawn_empty().id();
		HtmlDiffer::new("<h1>Title</h1>")
			.diff(world.entity_mut(root))
			.unwrap();

		let children = child_entities(&world, root);
		children.len().xpect_eq(1);
		world
			.entity(children[0])
			.contains::<Heading1>()
			.xpect_true();
		collect_text(&world, children[0]).xpect_eq("Title");
	}

	#[test]
	fn diff_spawns_strong() {
		let mut world = World::new();
		let root = world.spawn_empty().id();
		HtmlDiffer::new("<p>hello <strong>bold</strong> text</p>")
			.diff(world.entity_mut(root))
			.unwrap();

		let children = child_entities(&world, root);
		let para = children[0];
		let para_children = child_entities(&world, para);
		// "hello " + Important("bold") + " text"
		para_children.len().xpect_eq(3);
		world
			.entity(para_children[1])
			.contains::<Important>()
			.xpect_true();
	}

	#[test]
	fn diff_spawns_link() {
		let mut world = World::new();
		let root = world.spawn_empty().id();
		HtmlDiffer::new("<a href=\"https://example.com\">click</a>")
			.diff(world.entity_mut(root))
			.unwrap();

		let children = child_entities(&world, root);
		let link = world.entity(children[0]).get::<Link>().unwrap();
		link.href.as_str().xpect_eq("https://example.com");
		collect_text(&world, children[0]).xpect_eq("click");
	}

	// -- Diff (update in place) --

	#[test]
	fn diff_updates_text_in_place() {
		let mut world = World::new();
		let root = world.spawn_empty().id();

		HtmlDiffer::new("<p>original</p>")
			.diff(world.entity_mut(root))
			.unwrap();

		let children_before = child_entities(&world, root);
		let para_entity = children_before[0];

		HtmlDiffer::new("<p>updated</p>")
			.diff(world.entity_mut(root))
			.unwrap();

		let children_after = child_entities(&world, root);
		// Same entity reused
		children_after[0].xpect_eq(para_entity);
		collect_text(&world, children_after[0]).xpect_eq("updated");
	}

	#[test]
	fn diff_replaces_mismatched_kind() {
		let mut world = World::new();
		let root = world.spawn_empty().id();

		HtmlDiffer::new("<p>para</p>")
			.diff(world.entity_mut(root))
			.unwrap();
		let old_child = child_entities(&world, root)[0];

		HtmlDiffer::new("<h1>heading</h1>")
			.diff(world.entity_mut(root))
			.unwrap();

		let children = child_entities(&world, root);
		children.len().xpect_eq(1);
		// Old entity was despawned, new one created
		(children[0] != old_child).xpect_true();
		world
			.entity(children[0])
			.contains::<Heading1>()
			.xpect_true();
	}

	#[test]
	fn diff_adds_new_children() {
		let mut world = World::new();
		let root = world.spawn_empty().id();

		HtmlDiffer::new("<p>one</p>")
			.diff(world.entity_mut(root))
			.unwrap();
		child_entities(&world, root).len().xpect_eq(1);

		HtmlDiffer::new("<p>one</p><p>two</p>")
			.diff(world.entity_mut(root))
			.unwrap();
		child_entities(&world, root).len().xpect_eq(2);
	}

	#[test]
	fn diff_removes_extra_children() {
		let mut world = World::new();
		let root = world.spawn_empty().id();

		HtmlDiffer::new("<p>one</p><p>two</p><p>three</p>")
			.diff(world.entity_mut(root))
			.unwrap();
		child_entities(&world, root).len().xpect_eq(3);

		HtmlDiffer::new("<p>one</p>")
			.diff(world.entity_mut(root))
			.unwrap();
		child_entities(&world, root).len().xpect_eq(1);
	}

	// -- Render roundtrip --

	#[test]
	fn render_roundtrip_paragraph() {
		let mut world = World::new();
		let root = world.spawn(CardTool).id();
		HtmlDiffer::new("<p>hello world</p>")
			.diff(world.entity_mut(root))
			.unwrap();
		render(&mut world, root).xpect_eq("<p>hello world</p>");
	}

	#[test]
	fn render_roundtrip_heading() {
		let mut world = World::new();
		let root = world.spawn(CardTool).id();
		HtmlDiffer::new("<h1>Title</h1>")
			.diff(world.entity_mut(root))
			.unwrap();
		render(&mut world, root).xpect_eq("<h1>Title</h1>");
	}

	#[test]
	fn render_roundtrip_inline_styles() {
		let mut world = World::new();
		let root = world.spawn(CardTool).id();
		HtmlDiffer::new(
			"<p>hello <strong>bold</strong> and <em>italic</em></p>",
		)
		.diff(world.entity_mut(root))
		.unwrap();
		render(&mut world, root)
			.xpect_eq("<p>hello <strong>bold</strong> and <em>italic</em></p>");
	}

	#[test]
	fn render_roundtrip_link() {
		let mut world = World::new();
		let root = world.spawn(CardTool).id();
		HtmlDiffer::new("<p><a href=\"https://example.com\">click</a></p>")
			.diff(world.entity_mut(root))
			.unwrap();
		render(&mut world, root)
			.xpect_eq("<p><a href=\"https://example.com\">click</a></p>");
	}

	#[test]
	fn render_roundtrip_code_block() {
		let mut world = World::new();
		let root = world.spawn(CardTool).id();
		HtmlDiffer::new(
			"<pre><code class=\"language-rust\">fn main() {}</code></pre>",
		)
		.diff(world.entity_mut(root))
		.unwrap();
		render(&mut world, root).xpect_eq(
			"<pre><code class=\"language-rust\">fn main() {}</code></pre>",
		);
	}

	#[test]
	fn render_roundtrip_list() {
		let mut world = World::new();
		let root = world.spawn(CardTool).id();
		HtmlDiffer::new("<ul><li>one</li><li>two</li></ul>")
			.diff(world.entity_mut(root))
			.unwrap();
		render(&mut world, root).xpect_eq("<ul><li>one</li><li>two</li></ul>");
	}

	#[test]
	fn render_roundtrip_blockquote() {
		let mut world = World::new();
		let root = world.spawn(CardTool).id();
		HtmlDiffer::new("<blockquote><p>quoted</p></blockquote>")
			.diff(world.entity_mut(root))
			.unwrap();
		render(&mut world, root)
			.xpect_eq("<blockquote><p>quoted</p></blockquote>");
	}

	#[test]
	fn render_roundtrip_complex() {
		let mut world = World::new();
		let root = world.spawn(CardTool).id();
		let html = "<h1>Welcome</h1><p>This is <strong>bold</strong> and <em>italic</em>.</p><ul><li>one</li><li>two</li></ul><hr />";
		HtmlDiffer::new(html).diff(world.entity_mut(root)).unwrap();
		render(&mut world, root).xpect_eq(html);
	}

	#[test]
	fn html_entity_decoding_in_text() {
		let mut world = World::new();
		let root = world.spawn(CardTool).id();
		HtmlDiffer::new("<p>A &amp; B &lt; C</p>")
			.diff(world.entity_mut(root))
			.unwrap();
		let children = child_entities(&world, root);
		collect_text(&world, children[0]).xpect_eq("A & B < C");
	}

	#[test]
	fn html_entity_roundtrip() {
		let mut world = World::new();
		let root = world.spawn(CardTool).id();
		// After decode + render, entities should be re-escaped
		HtmlDiffer::new("<p>A &amp; B</p>")
			.diff(world.entity_mut(root))
			.unwrap();
		render(&mut world, root).xpect_eq("<p>A &amp; B</p>");
	}

	#[test]
	fn parse_strikethrough() {
		let mut world = World::new();
		let root = world.spawn(CardTool).id();
		HtmlDiffer::new("<p><del>deleted</del></p>")
			.diff(world.entity_mut(root))
			.unwrap();
		render(&mut world, root).xpect_eq("<p><del>deleted</del></p>");
	}

	#[test]
	fn parse_hard_break() {
		let mut world = World::new();
		let root = world.spawn(CardTool).id();
		HtmlDiffer::new("<p>line one<br />line two</p>")
			.diff(world.entity_mut(root))
			.unwrap();
		render(&mut world, root).xpect_eq("<p>line one<br />line two</p>");
	}

	#[test]
	fn parse_thematic_break() {
		let mut world = World::new();
		let root = world.spawn(CardTool).id();
		HtmlDiffer::new("<p>above</p><hr /><p>below</p>")
			.diff(world.entity_mut(root))
			.unwrap();
		render(&mut world, root).xpect_eq("<p>above</p><hr /><p>below</p>");
	}
}
