use crate::prelude::*;
use alloc::borrow::Cow;
use beet_core::prelude::*;

/// Renders an entity tree back to a markdown string via [`NodeVisitor`].
///
/// Converts HTML-like element trees (as produced by [`MarkdownParser`])
/// back into CommonMark-compatible markdown. Supports headings, emphasis,
/// strong, links, images, lists, task list checkboxes, code blocks,
/// blockquotes, thematic breaks, inline code, GFM tables with their captions,
/// and optional expression rendering. A `<mark>`, and a `<span>` carrying a
/// `style`, pass through as inline HTML, since markdown has no syntax for what
/// they signal. A root carrying [`PageMeta`] leads with the YAML frontmatter
/// declaring it, so a parsed markdown file renders its metadata back and a
/// Word file its core properties.
///
/// A tree read from a richer format reads as a person would write it: an
/// inline wrapper's edge whitespace sits outside its marker, two adjacent
/// wrappers of one look read as one, an empty wrapper or block writes
/// nothing, and a table nested in a cell, which GFM cannot hold, follows its
/// host, named in the cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownRenderer {
	/// Shared block/inline tracking state and output buffer.
	state: TextRenderState,
	/// Render [`Expression`] values verbatim as `{expr}` in output.
	render_expressions: bool,
	/// The inline elements open in the walk, innermost last, each with the
	/// markers it writes.
	inline_stack: Vec<InlineMark>,
	/// Inline wrappers opened but not yet written: written before the first
	/// words inside them, after their leading whitespace, and dropped when
	/// nothing is.
	pending_opens: Vec<InlineMark>,
	/// Inline wrappers closed but not yet written: written before whatever
	/// follows, after hoisting the trailing whitespace, and cancelled when
	/// the same wrapper opens again at once.
	pending_closes: Vec<InlineMark>,
	/// The blocks open in the walk, innermost last, so an empty one is
	/// unwritten.
	blocks: Vec<BlockStart>,
	/// The tables being collected, innermost last.
	tables: Vec<TableCapture>,
	/// A checkbox was just written, so the words after it are spaced from
	/// it unless they bring their own space.
	after_box: bool,
	/// A block's prefix was just written, so the words after it lose their
	/// leading whitespace.
	at_block_start: bool,
}

/// An inline element's markers, ie `**` and `**` for `<strong>`; a wrapper
/// that signals nothing, ie an unstyled `<span>`, writes none.
#[derive(Debug, Clone, PartialEq, Eq)]
struct InlineMark {
	wrapper: InlineWrapper,
	open: String,
	close: String,
}

impl InlineMark {
	fn new(
		wrapper: InlineWrapper,
		open: impl Into<String>,
		close: impl Into<String>,
	) -> Self {
		Self {
			wrapper,
			open: open.into(),
			close: close.into(),
		}
	}

	/// A wrapper written by its own handling, not deferred.
	fn eager(wrapper: InlineWrapper) -> Self { Self::new(wrapper, "", "") }

	fn is_deferred(&self) -> bool { !self.open.is_empty() }
}

/// Where a block began, to unwrite it when it holds nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
struct BlockStart {
	/// The buffer before its separator.
	start: usize,
	/// The buffer after its prefix, ie `## `.
	content: usize,
	/// Whether a separator was pending before it.
	needed_separator: bool,
	/// Whether the buffer ended a line before it.
	trailing_newline: bool,
}

/// A table being collected, written as GFM when it closes, since a row is
/// only known once every cell in it is.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct TableCapture {
	/// Where the table starts in the buffer.
	start: usize,
	/// The rows so far, each cell's text.
	rows: Vec<Vec<String>>,
	/// Where the open cell's text starts in the buffer, and its column span.
	cell: Option<(usize, usize)>,
	/// Where the open caption starts in the buffer.
	caption_start: Option<usize>,
	/// The caption, written before the table.
	caption: String,
	/// The tables nested in this one's cells, written after it.
	nested: Vec<String>,
}

impl TableCapture {
	/// A cell's rendered text as one GFM cell: its lines joined by `<br>`,
	/// its pipes escaped.
	fn cell_text(text: &str) -> String {
		text.lines()
			.map(str::trim)
			.filter(|line| !line.is_empty())
			.collect::<Vec<_>>()
			.join("<br>")
			.replace('|', "\\|")
	}

	/// The table as GFM, its first row the header, every row padded to the
	/// widest, its caption before it and its nested tables after.
	fn to_markdown(&self) -> String {
		let width = self.rows.iter().map(Vec::len).max().unwrap_or(0);
		let mut out = String::new();
		if !self.caption.is_empty() {
			out.push_str(&self.caption);
			out.push_str("\n\n");
		}
		if width > 0 {
			let line = |row: &Vec<String>| {
				let cells = (0..width)
					.map(|index| {
						row.get(index).map(String::as_str).unwrap_or_default()
					})
					.collect::<Vec<_>>();
				format!("| {} |\n", cells.join(" | "))
			};
			out.push_str(&line(&self.rows[0]));
			out.push_str(&format!("|{}\n", "---|".repeat(width)));
			for row in &self.rows[1..] {
				out.push_str(&line(row));
			}
		}
		for nested in &self.nested {
			out.push('\n');
			out.push_str(nested);
		}
		out
	}

	/// What a cell holding this table shows in its place: its caption's
	/// words, a comment's without its markers, ie `[table t5]`.
	fn placeholder(&self) -> String {
		let named = self
			.caption
			.trim()
			.trim_start_matches("<!--")
			.trim_end_matches("-->")
			.trim();
		match named.is_empty() {
			true => "[table]".into(),
			false => format!("[table {named}]"),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InlineWrapper {
	Em,
	Strong,
	Del,
	InlineCode,
	/// Link wrapper deferred until leave.
	Link,
	/// Superscript.
	Sup,
	/// Subscript.
	Sub,
	/// Inline HTML, ie `<mark>`.
	Html,
	/// An element that writes nothing, ie an unstyled `<span>`.
	Transparent,
}

impl Default for MarkdownRenderer {
	fn default() -> Self { Self::new() }
}

impl MarkdownRenderer {
	/// The blocks unwritten when they hold nothing.
	const SUPPRESSIBLE: &[&str] =
		&["p", "h1", "h2", "h3", "h4", "h5", "h6", "li"];

	pub fn new() -> Self {
		Self {
			state: TextRenderState::new(),
			render_expressions: false,
			inline_stack: Vec::new(),
			pending_opens: Vec::new(),
			pending_closes: Vec::new(),
			blocks: Vec::new(),
			tables: Vec::new(),
			after_box: false,
			at_block_start: false,
		}
	}

	/// Enable rendering of [`Expression`] nodes as `{expr}`.
	pub fn with_expressions(mut self) -> Self {
		self.render_expressions = true;
		self
	}

	/// Override the set of block-level elements.
	pub fn with_block_elements(
		mut self,
		elements: Vec<Cow<'static, str>>,
	) -> Self {
		self.state = self.state.with_block_elements(elements);
		self
	}

	/// Consume the renderer and return the accumulated markdown string.
	pub fn into_string(self) -> String { self.state.buffer }

	/// Borrow the accumulated markdown string.
	pub fn as_str(&self) -> &str { &self.state.buffer }

	fn push_str(&mut self, text: &str) { self.state.push_raw(text); }

	fn push_char(&mut self, ch: char) { self.state.push_raw_char(ch); }

	/// Opens an inline wrapper, deferred until words are written inside it,
	/// or continuing the identical wrapper that just closed.
	fn open_inline(&mut self, mark: InlineMark) {
		if self.pending_closes.last() == Some(&mark) {
			self.pending_closes.pop();
			self.inline_stack.push(mark);
			return;
		}
		self.flush_closes();
		self.pending_opens.push(mark.clone());
		self.inline_stack.push(mark);
	}

	/// Closes the innermost inline element: an unwritten one vanishes, a
	/// written one closes once whatever follows is known.
	fn close_inline(&mut self, wrapper: InlineWrapper) -> Option<InlineMark> {
		let position = self
			.inline_stack
			.iter()
			.rposition(|mark| mark.wrapper == wrapper)?;
		let mark = self.inline_stack.remove(position);
		if !mark.is_deferred() {
			return Some(mark);
		}
		match self.pending_opens.last() == Some(&mark) {
			true => {
				self.pending_opens.pop();
			}
			false => self.pending_closes.push(mark.clone()),
		}
		Some(mark)
	}

	/// Writes the closes pending, each wrapper's trailing whitespace moved
	/// after its marker, so `**Date **` reads `**Date** `.
	fn flush_closes(&mut self) {
		for mark in core::mem::take(&mut self.pending_closes) {
			let buffer = &self.state.buffer;
			let kept = buffer.trim_end_matches([' ', '\t', '\u{a0}']).len();
			let trailing = self.state.buffer.split_off(kept);
			self.push_str(&mark.close);
			self.push_str(&trailing);
		}
	}

	/// Writes every pending marker before something that is no words, ie a
	/// line break, an image or a block.
	fn flush_inline(&mut self) {
		self.after_box = false;
		self.at_block_start = false;
		self.flush_closes();
		for mark in core::mem::take(&mut self.pending_opens) {
			self.push_str(&mark.open);
		}
	}

	/// Writes words, the pending opens after their leading whitespace and
	/// before the rest, whitespace alone leaving them pending.
	fn push_words(&mut self, text: &str) {
		let text = match self.at_block_start {
			true => text.trim_start(),
			false => text,
		};
		if text.is_empty() {
			return;
		}
		self.at_block_start = false;
		if core::mem::take(&mut self.after_box)
			&& !text.starts_with(char::is_whitespace)
		{
			self.push_char(' ');
		}
		self.flush_closes();
		if self.pending_opens.is_empty() {
			self.push_str(text);
			return;
		}
		let rest = text.trim_start_matches([' ', '\t', '\u{a0}', '\n']);
		let lead = &text[..text.len() - rest.len()];
		self.push_str(lead);
		if rest.is_empty() {
			return;
		}
		for mark in core::mem::take(&mut self.pending_opens) {
			self.push_str(&mark.open);
		}
		self.push_str(rest);
	}

	/// Opens an inline HTML element, its `style` kept: `mark` always, any
	/// other only when styled, since an unstyled `span` signals nothing.
	fn open_html(&mut self, view: &ElementView, close: &'static str) {
		let style = view.attribute_string("style");
		let open = match (view.tag(), style.is_empty()) {
			("mark", true) => "<mark>".to_string(),
			(tag, false) => {
				format!("<{tag} style=\"{}\">", style.replace('"', "&quot;"))
			}
			_ => {
				self.inline_stack
					.push(InlineMark::eager(InlineWrapper::Transparent));
				return;
			}
		};
		self.open_inline(InlineMark::new(InlineWrapper::Html, open, close));
	}

	/// Records a suppressible block's start before its separator.
	fn start_block(&mut self) -> BlockStart {
		BlockStart {
			start: self.state.buffer.len(),
			content: self.state.buffer.len(),
			needed_separator: self.state.needs_block_separator,
			trailing_newline: self.state.trailing_newline,
		}
	}

	/// Closes a suppressible block, unwriting it when it wrote nothing past
	/// its prefix. Answers whether it was kept.
	fn end_block(&mut self) -> bool {
		self.flush_closes();
		let Some(block) = self.blocks.pop() else {
			return true;
		};
		// trailing spaces, or a line break, say nothing at a block's end
		let kept = self
			.state
			.buffer
			.trim_end_matches([' ', '\t', '\u{a0}', '\n'])
			.len()
			.max(block.content);
		if kept < self.state.buffer.len() {
			self.state.buffer.truncate(kept);
			self.state.trailing_newline = self.state.buffer.ends_with('\n')
				|| self.state.buffer.is_empty();
		}
		let empty = self
			.state
			.buffer
			.get(block.content..)
			.is_some_and(|written| written.trim().is_empty());
		if empty {
			self.state.buffer.truncate(block.start);
			self.state.needs_block_separator = block.needed_separator;
			self.state.trailing_newline = block.trailing_newline;
		}
		!empty
	}
}

impl NodeVisitor for MarkdownRenderer {
	fn visit_element(&mut self, cx: &VisitContext, view: ElementView) {
		let name = view.tag();
		let value = view.value;
		let inline = matches!(
			name,
			"em" | "i"
				| "strong" | "b"
				| "del" | "s"
				| "sup" | "sub"
				| "mark" | "span"
		);
		// whether the block around has written nothing yet
		let block_start = self.at_block_start;
		// a line break before a block's first words writes nothing, so the
		// wrappers around it stay unwritten, ie a bold page break
		if !inline && !(name == "br" && block_start) {
			self.flush_inline();
		}
		let block = Self::SUPPRESSIBLE
			.contains(&name)
			.then(|| self.start_block());
		let is_checkbox =
			name == "input" && view.attribute_string("type") == "checkbox";

		match name {
			// ── Headings ──
			"h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
				self.state.ensure_block_separator();
				let level = name.as_bytes()[1] - b'0';
				let prefix = "#".repeat(level as usize);
				self.push_str(&format!("{prefix} "));
			}

			// ── Paragraph ──
			"p" => {
				self.state.ensure_block_separator_with_prefix(Some("> "));
				// emit blockquote prefix immediately so inline elements
				// (eg <em>) that open before the first text node are
				// correctly placed after the prefix
				if self.state.blockquote_depth > 0 {
					let prefix = self.state.blockquote_prefix("> ");
					self.push_str(&prefix);
				}
			}

			// ── Blockquote ──
			"blockquote" => {
				self.state.ensure_block_separator_with_prefix(Some("> "));
				self.state.blockquote_depth += 1;
			}

			// ── Lists ──
			"ul" => {
				self.state.enter_ul();
			}
			"ol" => {
				let start = view.try_as::<OrderedListView>().unwrap().start;
				self.state.enter_ol(start);
			}
			"li" => {
				self.state.ensure_newline();
				self.state.write_list_indent();
				let prefix = self.state.next_list_prefix("- ");
				self.push_str(&prefix);
			}

			// ── Code blocks ──
			"pre" => {
				self.state.ensure_block_separator();
				self.state.in_preformatted = true;
			}
			"code" if self.state.in_preformatted => {
				// fenced code block: the whole info string where the parser
				// kept it, else the language from the class
				let info = Some(view.attribute_string("data-info"))
					.filter(|info| !info.is_empty())
					.or_else(|| {
						view.attribute("class").and_then(|attr| {
							match attr.value {
								Value::Str(class) => class
									.strip_prefix("language-")
									.map(|lang| lang.to_string())
									.or_else(|| Some(class.to_string())),
								_ => None,
							}
						})
					})
					.unwrap_or_default();
				self.state.code_fence_info = Some(info.clone());
				self.push_str("```");
				self.push_str(&info);
				self.push_char('\n');
			}
			"code" => {
				// inline code
				self.push_char('`');
				self.inline_stack
					.push(InlineMark::eager(InlineWrapper::InlineCode));
			}

			// ── Inline formatting ──
			"em" | "i" => {
				self.open_inline(InlineMark::new(InlineWrapper::Em, "*", "*"));
			}
			"strong" | "b" => {
				self.open_inline(InlineMark::new(
					InlineWrapper::Strong,
					"**",
					"**",
				));
			}
			"del" | "s" => {
				self.open_inline(InlineMark::new(
					InlineWrapper::Del,
					"~~",
					"~~",
				));
			}
			"sup" => {
				self.open_inline(InlineMark::new(InlineWrapper::Sup, "^", "^"));
			}
			"sub" => {
				self.open_inline(InlineMark::new(InlineWrapper::Sub, "~", "~"));
			}

			// ── Links ──
			"a" => {
				let href = view.attribute_string("href");
				self.state.pending_link_href = Some(href);
				self.push_char('[');
				self.inline_stack
					.push(InlineMark::eager(InlineWrapper::Link));
			}

			// ── Images (void element) ──
			"img" => {
				let src = view.attribute_string("src");
				let alt = view.attribute_string("alt");
				self.push_str(&format!("![{}]({})", alt, src));
			}

			// ── A checkbox, as a task list writes it ──
			"input" if is_checkbox => {
				let checked = matches!(value, Some(Value::Bool(true)))
					|| view.attribute("checked").is_some();
				self.push_str(if checked { "[x]" } else { "[ ]" });
				self.after_box = true;
			}

			// ── Thematic break ──
			"hr" => {
				self.state.ensure_block_separator();
				self.push_str("---");
				self.state.ensure_newline();
				self.state.needs_block_separator = true;
			}

			// ── Line break, none before a block's first words ──
			"br" if !block_start => {
				self.push_str("  \n");
			}
			"br" => {}

			// ── Inline HTML ──
			"mark" => self.open_html(&view, "</mark>"),
			"span" => self.open_html(&view, "</span>"),

			// ── Tables, collected and written on leave ──
			"table" => {
				// a table in a cell is GFM's to hold nowhere, so it follows
				// its host
				if self.tables.last().is_none_or(|table| table.cell.is_none()) {
					self.state.ensure_block_separator();
				}
				self.tables.push(TableCapture {
					start: self.state.buffer.len(),
					..default()
				});
			}
			"caption" => {
				let start = self.state.buffer.len();
				if let Some(table) = self.tables.last_mut() {
					table.caption_start = Some(start);
				}
			}
			"tr" => {
				if let Some(table) = self.tables.last_mut() {
					table.rows.push(Vec::new());
				}
			}
			"th" | "td" => {
				let span = view
					.attribute_string("colspan")
					.parse::<usize>()
					.unwrap_or(1)
					.max(1);
				let start = self.state.buffer.len();
				if let Some(table) = self.tables.last_mut() {
					table.cell = Some((start, span));
				}
			}
			"thead" | "tbody" | "tfoot" => {}

			// ── Catch-all for unknown block/inline elements ──
			_ => {
				if self.state.is_block_element(name) {
					self.state.ensure_block_separator();
				}
			}
		}
		if let Some(mut block) = block {
			block.content = self.state.buffer.len();
			self.blocks.push(block);
			self.at_block_start = true;
		}
		// a control's typed value is its text, a checkbox's its box
		if let Some(value) = value
			&& !is_checkbox
		{
			self.visit_value(cx, value);
		}
	}

	fn leave_element(&mut self, _cx: &VisitContext, element: &Element) {
		let name = element.tag();
		let inline = matches!(
			name,
			"em" | "i"
				| "strong" | "b"
				| "del" | "s"
				| "sup" | "sub"
				| "mark" | "span"
		);
		if inline {
			let wrapper = match name {
				"em" | "i" => InlineWrapper::Em,
				"strong" | "b" => InlineWrapper::Strong,
				"del" | "s" => InlineWrapper::Del,
				"sup" => InlineWrapper::Sup,
				"sub" => InlineWrapper::Sub,
				"mark" => InlineWrapper::Html,
				_ => match self.inline_stack.last() {
					Some(mark)
						if mark.wrapper == InlineWrapper::Transparent =>
					{
						InlineWrapper::Transparent
					}
					_ => InlineWrapper::Html,
				},
			};
			self.close_inline(wrapper);
			return;
		}
		let kept = match Self::SUPPRESSIBLE.contains(&name) {
			true => self.end_block(),
			// a void element wrote all it writes on its visit
			false if matches!(name, "input" | "img" | "br" | "hr") => true,
			false => {
				self.flush_inline();
				true
			}
		};

		match name {
			// ── Headings ──
			"h1" | "h2" | "h3" | "h4" | "h5" | "h6" if kept => {
				self.state.ensure_newline();
				self.state.needs_block_separator = true;
			}

			// ── Paragraph ──
			"p" if kept => {
				self.state.ensure_newline();
				self.state.needs_block_separator = true;
			}

			// ── Blockquote ──
			"blockquote" => {
				self.state.blockquote_depth =
					self.state.blockquote_depth.saturating_sub(1);
				self.state.ensure_newline();
				self.state.needs_block_separator = true;
			}

			// ── Lists ──
			"ul" | "ol" => {
				self.state.leave_list();
				if self.state.list_depth == 0 {
					self.state.ensure_newline();
					self.state.needs_block_separator = true;
				}
			}
			"li" if kept => {
				self.state.ensure_newline();
			}

			// ── Code blocks ──
			"code" if self.state.in_preformatted => {
				self.state.ensure_newline();
				self.push_str("```");
				self.push_char('\n');
				self.state.code_fence_info = None;
			}
			"code" => {
				self.close_inline(InlineWrapper::InlineCode);
				self.push_char('`');
			}
			"pre" => {
				self.state.in_preformatted = false;
				self.state.needs_block_separator = true;
			}

			// ── Links ──
			"a" => {
				self.close_inline(InlineWrapper::Link);
				let href =
					self.state.pending_link_href.take().unwrap_or_default();
				self.push_str("](");
				self.push_str(&href);
				self.push_char(')');
			}

			// ── Tables ──
			"caption" => {
				let Some(start) = self
					.tables
					.last_mut()
					.and_then(|table| table.caption_start.take())
				else {
					return;
				};
				let caption = self.state.take_from(start).trim().to_string();
				self.state.needs_block_separator = false;
				if let Some(table) = self.tables.last_mut() {
					table.caption = caption;
				}
			}
			"th" | "td" => {
				let Some((start, span)) =
					self.tables.last_mut().and_then(|table| table.cell.take())
				else {
					return;
				};
				let text =
					TableCapture::cell_text(&self.state.take_from(start));
				self.state.needs_block_separator = false;
				if let Some(table) = self.tables.last_mut() {
					if table.rows.is_empty() {
						table.rows.push(Vec::new());
					}
					let row = table.rows.last_mut().unwrap();
					row.push(text);
					row.extend((1..span).map(|_| String::new()));
				}
			}
			"table" => {
				let Some(table) = self.tables.pop() else {
					return;
				};
				// whatever the table held outside its cells is dropped
				self.state.take_from(table.start);
				match self.tables.last_mut() {
					// nested in a cell: named there, written after its host
					Some(host) if host.cell.is_some() => {
						host.nested.push(table.to_markdown());
						self.push_str(&table.placeholder());
					}
					_ => {
						self.push_str(&table.to_markdown());
						self.state.ensure_newline();
						self.state.needs_block_separator = true;
					}
				}
			}
			"tr" | "thead" | "tbody" | "tfoot" => {}

			// ── Images (void element, fully handled in visit_element) ──
			"img" | "input" => {}

			// ── Void/self-closing ──
			"hr" | "br" => {
				// already handled in visit_element
			}

			_ => {
				if kept && self.state.is_block_element(name) {
					self.state.ensure_newline();
					self.state.needs_block_separator = true;
				}
			}
		}
	}

	fn visit_value(&mut self, _cx: &VisitContext, value: &Value) {
		let text = value.to_string();
		self.push_words(&text);
	}

	fn visit_expression(
		&mut self,
		_cx: &VisitContext,
		expression: &Expression,
	) {
		if self.render_expressions {
			self.flush_inline();
			self.push_char('{');
			self.push_str(&expression.0);
			self.push_char('}');
		}
	}

	fn visit_comment(&mut self, _cx: &VisitContext, comment: &Comment) {
		self.flush_inline();
		self.state.ensure_block_separator();
		self.push_str("<!--");
		self.push_str(comment);
		self.push_str("-->");
		self.state.ensure_newline();
		self.state.needs_block_separator = true;
	}
}

impl NodeRenderer for MarkdownRenderer {
	fn render(
		&mut self,
		cx: &mut RenderContext,
	) -> Result<MediaBytes, RenderError> {
		cx.check_accepts(&[MediaType::Markdown])?;
		// a document's metadata leads it as the frontmatter that declares it
		#[cfg(feature = "bsx")]
		if let Some(block) = cx
			.world
			.get::<PageMeta>(cx.entity)
			.map(Frontmatter::write)
			.transpose()?
			.flatten()
		{
			self.push_str(&block);
			self.state.needs_block_separator = true;
		}
		cx.walk(self);
		self.flush_inline();
		MediaBytes::new_string(
			MediaType::Markdown,
			core::mem::take(&mut self.state.buffer),
		)
		.xok()
	}
}

#[cfg(test)]
#[cfg(feature = "markdown_parser")]
mod test {
	use super::*;

	/// Parse markdown then render it back via [`MarkdownRenderer`].
	fn roundtrip(md: &str) -> String {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		let bytes = MediaBytes::new_markdown(md);
		MarkdownParser::new()
			.parse(ParseContext::new(&mut world.entity_mut(entity), &bytes))
			.unwrap();
		MarkdownRenderer::new()
			.render(&mut RenderContext::new(entity, &mut world))
			.unwrap()
			.to_string()
	}

	/// Parse markdown then render with expression support.
	#[allow(dead_code)]
	fn roundtrip_expressions(md: &str) -> String {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		let bytes = MediaBytes::new_markdown(md);
		MarkdownParser::with_expressions()
			.parse(ParseContext::new(&mut world.entity_mut(entity), &bytes))
			.unwrap();
		MarkdownRenderer::new()
			.with_expressions()
			.render(&mut RenderContext::new(entity, &mut world))
			.unwrap()
			.to_string()
	}

	#[beet_core::test]
	fn render_paragraph() {
		roundtrip("Hello world").trim().xpect_eq("Hello world");
	}

	#[beet_core::test]
	fn render_heading_h1() { roundtrip("# Title").trim().xpect_eq("# Title"); }

	#[beet_core::test]
	fn render_heading_h2() {
		roundtrip("## Subtitle").trim().xpect_eq("## Subtitle");
	}

	#[beet_core::test]
	fn render_emphasis() { roundtrip("*hello*").trim().xpect_eq("*hello*"); }

	#[beet_core::test]
	fn render_strong() { roundtrip("**hello**").trim().xpect_eq("**hello**"); }

	#[beet_core::test]
	fn render_link() {
		roundtrip("[click](https://example.com)")
			.trim()
			.xpect_eq("[click](https://example.com)");
	}

	#[beet_core::test]
	fn render_image() {
		roundtrip("![alt](image.png)")
			.trim()
			.xpect_eq("![alt](image.png)");
	}

	#[beet_core::test]
	fn render_unordered_list() {
		roundtrip("- alpha\n- beta")
			.trim()
			.xpect_eq("- alpha\n- beta");
	}

	#[beet_core::test]
	fn render_code_block() {
		roundtrip("```rust\nfn main() {}\n```")
			.trim()
			.xpect_eq("```rust\nfn main() {}\n```");
	}

	#[beet_core::test]
	fn render_inline_code() {
		roundtrip("use `foo()` here")
			.trim()
			.xpect_eq("use `foo()` here");
	}

	#[beet_core::test]
	fn render_blockquote() {
		roundtrip("> quoted text").trim().xpect_eq("> quoted text");
	}

	#[beet_core::test]
	fn render_blockquote_with_emphasis() {
		// inline elements inside a blockquote must appear after the prefix
		roundtrip("> *notable remark*")
			.trim()
			.xpect_eq("> *notable remark*");
	}

	#[beet_core::test]
	fn render_blockquote_multiline() {
		let input = "> first paragraph\n>\n> second paragraph";
		roundtrip(input).trim().xpect_eq(input);
	}

	#[beet_core::test]
	fn render_thematic_break() { roundtrip("---").trim().xpect_eq("---"); }

	#[beet_core::test]
	fn render_multiple_blocks() {
		roundtrip("# Title\n\nParagraph")
			.trim()
			.xpect_eq("# Title\n\nParagraph");
	}

	/// A root's metadata leads its render as the frontmatter declaring it.
	#[beet_core::test]
	fn renders_page_meta_as_frontmatter() {
		let mut world = World::new();
		let entity = world
			.spawn(PageMeta {
				title: Some("Plan".into()),
				..default()
			})
			.id();
		MarkdownParser::new()
			.parse(ParseContext::new(
				&mut world.entity_mut(entity),
				&MediaBytes::new_markdown("# Plan"),
			))
			.unwrap();
		MarkdownRenderer::new()
			.render(&mut RenderContext::new(entity, &mut world))
			.unwrap()
			.to_string()
			.xpect_eq("---\ntitle: Plan\n---\n\n# Plan\n");
	}

	#[beet_core::test]
	fn render_comment() {
		roundtrip("<!-- hello -->")
			.trim()
			.xpect_eq("<!-- hello -->");
	}

	#[cfg(feature = "bsx")]
	/// Parse HTML (with entities), then render as markdown.
	fn render_unescaped(html: &str) -> String {
		let mut world = world_ext::ui_world();
		let entity = world.spawn_empty().id();
		let bytes = MediaBytes::new_html(html);
		BsxParser::html()
			.parse(ParseContext::new(&mut world.entity_mut(entity), &bytes))
			.unwrap();
		MarkdownRenderer::new()
			.render(&mut RenderContext::new(entity, &mut world))
			.unwrap()
			.to_string()
	}

	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn unescape_html_entities_in_text() {
		render_unescaped("<p>a &amp; b</p>")
			.trim()
			.xpect_eq("a & b");
	}

	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn unescape_angle_brackets_in_text() {
		render_unescaped("<p>&lt;div&gt;</p>")
			.trim()
			.xpect_eq("<div>");
	}

	#[beet_core::test]
	fn render_table() {
		roundtrip("| a | b |\n|---|---|\n| c | d \\| e |")
			.trim()
			.xpect_eq("| a | b |\n|---|---|\n| c | d \\| e |");
	}

	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn render_html_table() {
		render_unescaped(
			"<p>before</p><table><tr><td><p>one</p><p>two</p></td><td colspan=\"2\">x</td></tr><tr><td>a</td></tr></table><p>after</p>",
		)
		.trim()
		.xpect_eq("before\n\n| one<br>two | x |  |\n|---|---|---|\n| a |  |  |\n\nafter");
	}

	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn render_inline_html() {
		render_unescaped(
			"<p><mark>due</mark> <span>plain</span> <span style=\"color: #FF0000\">red</span></p>",
		)
		.trim()
		.xpect_eq("<mark>due</mark> plain <span style=\"color: #FF0000\">red</span>");
	}

	/// A tree read from a richer format reads as a person writes markdown:
	/// edge whitespace outside its emphasis, two wrappers of one look as one,
	/// an empty wrapper or block unwritten.
	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn writes_inline_looks_as_a_person_would() {
		render_unescaped(
			"<p><strong>Date </strong>due<em> soon</em></p>\
			 <p><mark>(Insert</mark><mark> name)</mark> <strong>a</strong><strong>b</strong><strong></strong></p>\
			 <p></p><h2> </h2><p>last<br></p><p><strong><br></strong></p>",
		)
		.xpect_eq("**Date** due *soon*\n\n<mark>(Insert name)</mark> **ab**\n\nlast\n");
	}

	/// A table's caption sits above it, and a table nested in a cell, which
	/// GFM cannot hold, is named in the cell and follows its host.
	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn writes_captions_and_nested_tables() {
		render_unescaped(
			"<table><caption><!-- t1 --></caption><tr><td>a</td><td>\
			 <table><caption><!-- t2 --></caption><tr><td>x</td></tr></table>\
			 </td></tr></table>",
		)
		.xpect_eq(
			"<!-- t1 -->\n\n| a | [table t2] |\n|---|---|\n\n<!-- t2 -->\n\n| x |\n|---|\n",
		);
	}

	/// A checkbox reads as a task list writes it.
	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn writes_checkboxes() {
		roundtrip("- [x] done\n- [ ] not yet")
			.trim()
			.xpect_eq("- [x] done\n- [ ] not yet");
		render_unescaped("<p><input type=\"checkbox\"> Surveys</p>")
			.xpect_eq("[ ] Surveys\n");
	}

	#[beet_core::test]
	fn plain_text_passes_through() {
		roundtrip("hello world").trim().xpect_eq("hello world");
	}
}
