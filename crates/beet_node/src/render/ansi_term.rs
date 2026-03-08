use crate::prelude::*;
use beet_core::prelude::*;
use nu_ansi_term::Color;
use nu_ansi_term::Style;
use std::borrow::Cow;

/// Renders an entity tree to styled ANSI terminal output via [`NodeVisitor`].
///
/// Maps HTML-like element names to [`Style`] values. When an element has
/// no explicit style, [`AnsiTermRenderer::default_associations`] is checked
/// for a fallback element name whose style should be reused. If neither
/// map contains an entry, [`AnsiTermRenderer::default_style`] is used.
///
/// Block-level elements emit newlines following the same rules as HTML,
/// while inline elements are rendered contiguously. Anchor tags render
/// as [OSC-8 hyperlinks](https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda).
pub struct AnsiTermRenderer {
	/// Explicit element-name → style mapping.
	style_map: HashMap<Cow<'static, str>, Style>,
	/// Fallback mapping: if an element is missing from `style_map`,
	/// look up its association here and use that element's style instead.
	default_associations: HashMap<Cow<'static, str>, Cow<'static, str>>,
	/// Style used when no mapping or association is found.
	default_style: Style,
	/// Elements that produce block-level output with trailing newlines.
	block_tags: Vec<Cow<'static, str>>,
	/// Render [`Expression`] values verbatim as `{expr}`.
	render_expressions: bool,
	/// Whether to prefix headings with `#` markers.
	heading_hashes: bool,
	// ── Internal state ──
	buffer: String,
	/// Stack of active styles so nested elements restore correctly.
	style_stack: Vec<Style>,
	/// Whether the last character written was a newline.
	trailing_newline: bool,
	/// Whether a blank line separator is needed before the next block.
	needs_block_separator: bool,
	/// The href from the current `<a>` element, used for OSC-8 links.
	pending_link_href: Option<String>,
	/// When inside an `<img>`, the src URL from its attribute.
	/// Child text nodes are captured as alt text instead of being emitted.
	image_src: Option<String>,
	/// Accumulated alt text while inside an `<img>` element.
	image_alt: Option<String>,
	/// Track list contexts for bullets and numbering.
	list_stack: Vec<ListContext>,
	/// Nesting depth for lists.
	list_depth: usize,
	/// Whether we are inside a `<pre>` block.
	in_preformatted: bool,
	/// Pending prefix for the next text node, ie heading markers, list bullets.
	pending_prefix: Option<String>,
}

#[derive(Debug, Clone)]
enum ListContext {
	Unordered,
	Ordered(usize),
}

impl Default for AnsiTermRenderer {
	fn default() -> Self { Self::new() }
}

impl AnsiTermRenderer {
	pub fn new() -> Self {
		Self {
			style_map: default_style_map(),
			default_associations: default_associations(),
			default_style: Style::default(),
			block_tags: default_block_elements(),
			render_expressions: false,
			buffer: String::new(),
			style_stack: Vec::new(),
			trailing_newline: true,
			heading_hashes: false,
			needs_block_separator: false,
			pending_link_href: None,
			image_src: None,
			image_alt: None,
			list_stack: Vec::new(),
			list_depth: 0,
			in_preformatted: false,
			pending_prefix: None,
		}
	}

	/// Override the element → style mapping.
	pub fn with_style_map(
		mut self,
		map: HashMap<Cow<'static, str>, Style>,
	) -> Self {
		self.style_map = map;
		self
	}

	/// Override the fallback association mapping.
	pub fn with_default_associations(
		mut self,
		map: HashMap<Cow<'static, str>, Cow<'static, str>>,
	) -> Self {
		self.default_associations = map;
		self
	}

	/// Override the default style used when no mapping is found.
	pub fn with_default_style(mut self, style: Style) -> Self {
		self.default_style = style;
		self
	}

	/// Override the set of block-level tags.
	pub fn with_block_tags(mut self, tags: Vec<Cow<'static, str>>) -> Self {
		self.block_tags = tags;
		self
	}

	/// Enable rendering of [`Expression`] nodes as `{expr}`.
	pub fn with_expressions(mut self) -> Self {
		self.render_expressions = true;
		self
	}

	/// Consume the renderer and return the accumulated string.
	pub fn into_string(self) -> String { self.buffer }

	/// Borrow the accumulated string.
	pub fn as_str(&self) -> &str { &self.buffer }

	/// Resolve the style for an element name, walking through
	/// associations if needed.
	fn resolve_style(&self, name: &str) -> Style {
		let lower = name.to_ascii_lowercase();

		// direct lookup
		if let Some(style) = self.style_map.get(lower.as_str()) {
			return *style;
		}

		// association fallback (one level deep to avoid cycles)
		if let Some(assoc) = self.default_associations.get(lower.as_str()) {
			if let Some(style) = self.style_map.get(assoc.as_ref()) {
				return *style;
			}
		}

		self.default_style
	}

	fn current_style(&self) -> Style {
		self.style_stack
			.last()
			.copied()
			.unwrap_or(self.default_style)
	}

	fn is_block_tag(&self, name: &str) -> bool {
		let lower = name.to_ascii_lowercase();
		self.block_tags.iter().any(|el| el.as_ref() == lower)
	}

	fn ensure_newline(&mut self) {
		if !self.trailing_newline {
			self.buffer.push('\n');
			self.trailing_newline = true;
		}
	}

	fn ensure_block_separator(&mut self) {
		if self.needs_block_separator && !self.buffer.is_empty() {
			self.ensure_newline();
			if !self.buffer.ends_with("\n\n") {
				self.buffer.push('\n');
			}
		}
		self.needs_block_separator = false;
	}

	fn push_styled(&mut self, text: &str) {
		let style = self.current_style();
		let painted = format!("{}", style.paint(text));
		self.buffer.push_str(&painted);
		self.trailing_newline = text.ends_with('\n');
	}

	fn push_raw(&mut self, text: &str) {
		self.buffer.push_str(text);
		self.trailing_newline = text.ends_with('\n');
	}

	/// Write an OSC-8 hyperlink opening sequence.
	fn open_osc8_link(&mut self, href: &str) {
		// ESC ] 8 ; params ; URI ST
		self.buffer.push_str("\x1b]8;;");
		self.buffer.push_str(href);
		self.buffer.push_str("\x1b\\");
	}

	/// Write an OSC-8 hyperlink closing sequence.
	fn close_osc8_link(&mut self) { self.buffer.push_str("\x1b]8;;\x1b\\"); }

	fn write_list_indent(&mut self) {
		if self.list_depth > 1 {
			for _ in 0..self.list_depth - 1 {
				self.push_raw("  ");
			}
		}
	}

	fn find_attr<'a>(
		attrs: &'a [(Entity, &Attribute, &Value)],
		key: &str,
	) -> Option<&'a Value> {
		attrs
			.iter()
			.find(|(_, attr, _)| attr.as_str() == key)
			.map(|(_, _, val)| *val)
	}
}


impl NodeVisitor for AnsiTermRenderer {
	fn visit_element(
		&mut self,
		_cx: &VisitContext,
		element: &Element,
		attrs: Vec<(Entity, &Attribute, &Value)>,
	) {
		let name = element.name().to_ascii_lowercase();
		let style = self.resolve_style(&name);
		self.style_stack.push(style);

		match name.as_str() {
			// ── Headings ──
			"h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
				self.ensure_block_separator();
				if self.heading_hashes {
					let level = name[1..].parse::<usize>().unwrap_or(1);
					let prefix = "#".repeat(level);
					self.pending_prefix = Some(format!("{prefix} "));
				}
			}

			// ── Paragraph ──
			"p" => {
				self.ensure_block_separator();
			}

			// ── Blockquote ──
			"blockquote" => {
				self.ensure_block_separator();
				self.pending_prefix = Some("▌ ".to_string());
			}

			// ── Lists ──
			"ul" => {
				if self.list_depth == 0 {
					self.ensure_block_separator();
				}
				self.list_depth += 1;
				self.list_stack.push(ListContext::Unordered);
			}
			"ol" => {
				if self.list_depth == 0 {
					self.ensure_block_separator();
				}
				self.list_depth += 1;
				let start = Self::find_attr(&attrs, "start")
					.and_then(|val| match val {
						Value::Uint(num) => Some(*num as usize),
						Value::Int(num) => Some(*num as usize),
						_ => None,
					})
					.unwrap_or(1);
				self.list_stack.push(ListContext::Ordered(start));
			}
			"li" => {
				self.ensure_newline();
				self.write_list_indent();
				let prefix = match self.list_stack.last_mut() {
					Some(ListContext::Unordered) => "• ".to_string(),
					Some(ListContext::Ordered(num)) => {
						let prefix = format!("{}. ", num);
						*num += 1;
						prefix
					}
					None => "• ".to_string(),
				};
				self.push_styled(&prefix);
			}

			// ── Code blocks ──
			"pre" => {
				self.ensure_block_separator();
				self.in_preformatted = true;
			}
			"code" if self.in_preformatted => {
				// code block content rendered directly in visit_value
			}
			"code" => {
				// inline code, no special prefix needed
			}

			// ── Links ──
			"a" => {
				let href = Self::find_attr(&attrs, "href")
					.map(|val| val.to_string())
					.unwrap_or_default();
				self.pending_link_href = Some(href.clone());
				self.open_osc8_link(&href);
			}

			// ── Images ──
			"img" => {
				let src = Self::find_attr(&attrs, "src")
					.map(|val| val.to_string())
					.unwrap_or_default();
				self.image_src = Some(src);
				self.image_alt = Some(String::new());
			}

			// ── Thematic break ──
			"hr" => {
				self.ensure_block_separator();
				self.push_styled("────────────────────");
				self.push_raw("\n");
				self.needs_block_separator = true;
			}

			// ── Line break ──
			"br" => {
				self.push_raw("\n");
			}

			// ── Generic block handling ──
			_ => {
				if self.is_block_tag(&name) {
					self.ensure_block_separator();
				}
			}
		}
	}

	fn leave_element(&mut self, _cx: &VisitContext, element: &Element) {
		let name = element.name().to_ascii_lowercase();

		match name.as_str() {
			"h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
				self.ensure_newline();
				self.needs_block_separator = true;
			}
			"p" => {
				self.ensure_newline();
				self.needs_block_separator = true;
			}
			"blockquote" => {
				self.ensure_newline();
				self.needs_block_separator = true;
			}
			"ul" | "ol" => {
				self.list_depth = self.list_depth.saturating_sub(1);
				self.list_stack.pop();
				if self.list_depth == 0 {
					self.ensure_newline();
					self.needs_block_separator = true;
				}
			}
			"li" => {
				self.ensure_newline();
			}
			"pre" => {
				self.in_preformatted = false;
				self.ensure_newline();
				self.needs_block_separator = true;
			}
			"code" if !self.in_preformatted => {
				// inline code, no suffix needed
			}
			"a" => {
				self.close_osc8_link();
				self.pending_link_href = None;
			}
			"img" => {
				let src = self.image_src.take().unwrap_or_default();
				let alt = self.image_alt.take().unwrap_or_default();
				let style = self.current_style();
				let display = if alt.is_empty() {
					format!("[image: {src}]")
				} else {
					format!("[{alt}]")
				};
				// render as an OSC-8 link to the image src
				self.open_osc8_link(&src);
				self.push_raw(&format!("{}", style.paint(&display)));
				self.close_osc8_link();
			}
			"hr" | "br" => {
				// already handled in visit_element
			}
			_ => {
				if self.is_block_tag(&name) {
					self.ensure_newline();
					self.needs_block_separator = true;
				}
			}
		}

		self.style_stack.pop();
	}

	fn visit_value(&mut self, _cx: &VisitContext, value: &Value) {
		let text = value.to_string();
		if text.is_empty() {
			return;
		}

		// if inside an <img>, capture text as alt instead of emitting
		if let Some(ref mut alt) = self.image_alt {
			alt.push_str(&text);
			return;
		}

		// emit any pending prefix (heading marker, blockquote, etc.)
		if let Some(prefix) = self.pending_prefix.take() {
			self.ensure_newline();
			self.push_styled(&prefix);
		}

		self.push_styled(&text);
	}

	fn visit_expression(
		&mut self,
		_cx: &VisitContext,
		expression: &Expression,
	) {
		if self.render_expressions {
			let style = Style::new().italic();
			let painted =
				format!("{}", style.paint(format!("{{{}}}", expression.0)));
			self.push_raw(&painted);
		}
	}

	fn visit_comment(&mut self, _cx: &VisitContext, comment: &Comment) {
		self.ensure_block_separator();
		let style = Style::new().dimmed();
		let painted =
			format!("{}", style.paint(format!("<!--{}-->", &**comment)));
		self.push_raw(&painted);
		self.push_raw("\n");
		self.trailing_newline = true;
		self.needs_block_separator = true;
	}
}


fn default_style_map() -> HashMap<Cow<'static, str>, Style> {
	let mut map = HashMap::default();
	map.insert("h1".into(), Style::new().bold().fg(Color::Green));
	map.insert("h2".into(), Style::new().bold().fg(Color::Cyan));
	map.insert("h3".into(), Style::new().bold().fg(Color::Blue));
	map.insert("h4".into(), Style::new().bold().fg(Color::Magenta));
	map.insert("h5".into(), Style::new().bold());
	map.insert("h6".into(), Style::new().bold().dimmed());
	map.insert("p".into(), Style::default());
	map.insert("a".into(), Style::new().fg(Color::Blue).underline());
	map.insert("strong".into(), Style::new().bold());
	map.insert("em".into(), Style::new().italic());
	map.insert("del".into(), Style::new().strikethrough());
	map.insert("code".into(), Style::new().fg(Color::Yellow));
	map.insert("pre".into(), Style::new().fg(Color::Yellow).dimmed());
	map.insert("blockquote".into(), Style::new().italic().dimmed());
	map.insert("hr".into(), Style::new().dimmed());
	map.insert("img".into(), Style::new().fg(Color::Magenta).underline());
	map.insert("li".into(), Style::default());
	map
}

fn default_associations() -> HashMap<Cow<'static, str>, Cow<'static, str>> {
	let mut map = HashMap::default();
	map.insert("b".into(), "strong".into());
	map.insert("i".into(), "em".into());
	map.insert("s".into(), "del".into());
	map.insert("div".into(), "p".into());
	map.insert("span".into(), "p".into());
	map.insert("section".into(), "p".into());
	map.insert("article".into(), "p".into());
	map.insert("aside".into(), "blockquote".into());
	map.insert("nav".into(), "p".into());
	map.insert("header".into(), "p".into());
	map.insert("footer".into(), "p".into());
	map.insert("main".into(), "p".into());
	map.insert("dt".into(), "strong".into());
	map.insert("dd".into(), "p".into());
	map.insert("th".into(), "strong".into());
	map.insert("td".into(), "p".into());
	map.insert("sup".into(), "em".into());
	map.insert("sub".into(), "em".into());
	map
}




#[cfg(test)]
mod test {
	use super::*;

	/// Parse markdown then render via [`AnsiTermRenderer`].
	fn render(md: &[u8]) -> String {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		MarkdownParser::new()
			.parse(&mut world.entity_mut(entity), md, None)
			.unwrap();
		world
			.run_system_once(move |walker: NodeWalker| {
				let mut render = AnsiTermRenderer::new();
				walker.walk(&mut render, entity);
				render.into_string()
			})
			.unwrap()
	}

	fn strip_ansi(input: &str) -> String {
		// strip ANSI escape sequences including OSC-8
		let mut result = String::new();
		let mut chars = input.chars().peekable();
		while let Some(ch) = chars.next() {
			if ch == '\x1b' {
				match chars.peek() {
					// OSC sequence: ESC ] ... ST (ST = ESC \)
					Some(']') => {
						chars.next();
						loop {
							match chars.next() {
								Some('\x1b') => {
									// consume the backslash of ST
									chars.next();
									break;
								}
								Some('\x07') => break, // BEL terminator
								None => break,
								_ => {}
							}
						}
					}
					// CSI sequence: ESC [ ... final byte
					Some('[') => {
						chars.next();
						loop {
							match chars.next() {
								Some(ch)
									if ch.is_ascii_alphabetic()
										|| ch == 'm' =>
								{
									break;
								}
								None => break,
								_ => {}
							}
						}
					}
					_ => {
						// other escape, skip next char
						chars.next();
					}
				}
			} else {
				result.push(ch);
			}
		}
		result
	}

	fn trim(input: String) -> String { input.trim().to_string() }

	#[test]
	fn render_paragraph() {
		strip_ansi(&render(b"Hello world"))
			.xmap(trim)
			.xpect_eq("Hello world".to_string());
	}

	#[test]
	fn render_heading_h1() {
		strip_ansi(&render(b"# Title"))
			.xmap(trim)
			.xpect_contains("Title");
	}

	#[test]
	fn render_heading_styled() {
		render(b"# Title")
			// should contain ANSI escape codes for bold green
			.xpect_contains("\x1b[")
			.xpect_contains("Title");
	}

	#[test]
	fn render_link_has_osc8() {
		render(b"[click](https://example.com)")
			// should contain OSC-8 opening and closing sequences
			.xpect_contains("\x1b]8;;https://example.com\x1b\\")
			.xpect_contains("click")
			.xpect_contains("\x1b]8;;\x1b\\");
	}

	#[test]
	fn render_link_text_stripped() {
		strip_ansi(&render(b"[click](https://example.com)"))
			.xmap(trim)
			.xpect_contains("click");
	}

	#[test]
	fn render_code_block() {
		strip_ansi(&render(b"```rust\nfn main() {}\n```"))
			.xpect_contains("fn main() {}");
	}

	#[test]
	fn render_unordered_list() {
		strip_ansi(&render(b"- alpha\n- beta"))
			.xpect_contains("alpha")
			.xpect_contains("beta")
			.xpect_contains("•");
	}

	#[test]
	fn render_image() {
		strip_ansi(&render(b"![alt text](image.png)"))
			.xpect_contains("[alt text]");
	}

	#[test]
	fn render_image_has_osc8() {
		render(b"![alt](image.png)").xpect_contains("\x1b]8;;image.png\x1b\\");
	}

	#[test]
	fn render_blockquote() {
		strip_ansi(&render(b"> quoted text"))
			.xpect_contains("▌")
			.xpect_contains("quoted text");
	}

	#[test]
	fn render_thematic_break() {
		strip_ansi(&render(b"---")).xpect_contains("────");
	}

	#[test]
	fn render_multiple_blocks_separated() {
		strip_ansi(&render(b"# Title\n\nParagraph"))
			.xpect_contains("Title")
			.xpect_contains("Paragraph")
			// should have a blank line between blocks
			.xpect_contains("\n\n");
	}

	#[test]
	fn custom_style_map() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		MarkdownParser::new()
			.parse(&mut world.entity_mut(entity), b"# Hello", None)
			.unwrap();
		world
			.run_system_once(move |walker: NodeWalker| {
				let mut custom_map: HashMap<Cow<'static, str>, Style> =
					HashMap::default();
				custom_map.insert("h1".into(), Style::new().fg(Color::Red));
				let mut render =
					AnsiTermRenderer::new().with_style_map(custom_map);
				walker.walk(&mut render, entity);
				render.into_string()
			})
			.unwrap()
			// red foreground escape code
			.xpect_contains("\x1b[")
			.xpect_contains("Hello");
	}
}
