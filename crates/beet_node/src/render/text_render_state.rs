use crate::prelude::*;
use beet_core::prelude::*;
use std::borrow::Cow;

/// Shared rendering state used by both [`MarkdownRenderer`] and [`AnsiTermRenderer`].
///
/// Tracks block/inline structure, list nesting, blockquote depth, image
/// capture, and the output buffer. Renderers that need styled output (eg ANSI)
/// wrap the push methods and apply colour before delegating to [`push_raw`].
pub struct TextRenderState {
	/// Output buffer.
	pub buffer: String,
	/// Elements that produce block-level output with trailing newlines.
	pub block_elements: Vec<Cow<'static, str>>,
	/// Track nested list depth for indentation.
	pub list_depth: usize,
	/// Stack tracking list item contexts for ordered list numbering.
	pub list_stack: Vec<ListContext>,
	/// Whether we are inside a `<pre>` block (suppresses formatting).
	pub in_preformatted: bool,
	/// Whether a blank line is needed before the next block element.
	pub needs_block_separator: bool,
	/// Tracks whether the last character written was a newline.
	pub trailing_newline: bool,
	/// Pending href captured from the most recent `<a>` element.
	pub pending_link_href: Option<String>,
	/// When inside an `<img>`, the src URL from its attribute.
	pub image_src: Option<String>,
	/// Accumulated alt text while inside an `<img>` element.
	pub image_alt: Option<String>,
	/// Code fence info string from `<code>` inside `<pre>`.
	pub code_fence_info: Option<String>,
	/// Nesting depth of `<blockquote>` elements.
	pub blockquote_depth: usize,
}

/// Tracks the context of a list for bullets and numbering.
#[derive(Debug, Clone)]
pub enum ListContext {
	Unordered,
	Ordered(usize),
}

impl Default for TextRenderState {
	fn default() -> Self { Self::new() }
}

impl TextRenderState {
	pub fn new() -> Self {
		Self {
			buffer: String::new(),
			block_elements: default_block_elements(),
			list_depth: 0,
			list_stack: Vec::new(),
			in_preformatted: false,
			needs_block_separator: false,
			trailing_newline: true, // start of document counts as newline
			pending_link_href: None,
			image_src: None,
			image_alt: None,
			code_fence_info: None,
			blockquote_depth: 0,
		}
	}

	/// Override the set of block-level elements.
	pub fn with_block_elements(
		mut self,
		elements: Vec<Cow<'static, str>>,
	) -> Self {
		self.block_elements = elements;
		self
	}

	pub fn is_block_element(&self, name: &str) -> bool {
		let lower = name.to_ascii_lowercase();
		self.block_elements.iter().any(|el| el.as_ref() == lower)
	}

	/// Ensure the buffer ends with a newline.
	pub fn ensure_newline(&mut self) {
		if !self.trailing_newline {
			self.buffer.push('\n');
			self.trailing_newline = true;
		}
	}

	/// Emit a blank-line block separator if one is pending.
	pub fn ensure_block_separator(&mut self) {
		self.ensure_block_separator_with_prefix(None);
	}

	/// Emit a blank-line block separator, prefixing the blank line with the
	/// blockquote marker when inside a blockquote.
	///
	/// `marker` is the per-level prefix string, eg `"> "` or `"▌ "`.
	/// Pass `None` when outside a blockquote or when no prefix is needed.
	pub fn ensure_block_separator_with_prefix(&mut self, marker: Option<&str>) {
		if self.needs_block_separator && !self.buffer.is_empty() {
			self.ensure_newline();
			if !self.buffer.ends_with("\n\n") {
				if let Some(marker) =
					marker.filter(|_| self.blockquote_depth > 0)
				{
					// trim trailing spaces so blank blockquote lines are
					// eg `>\n` not `> \n`
					let prefix = self.blockquote_prefix(marker);
					self.buffer.push_str(prefix.trim_end());
					self.buffer.push('\n');
				} else {
					self.buffer.push('\n');
				}
			}
		}
		self.needs_block_separator = false;
	}

	/// Write text directly to the buffer, updating newline tracking.
	pub fn push_raw(&mut self, text: &str) {
		self.buffer.push_str(text);
		if !text.is_empty() {
			self.trailing_newline = text.ends_with('\n');
		}
	}

	/// Write a single character directly to the buffer.
	pub fn push_raw_char(&mut self, ch: char) {
		self.buffer.push(ch);
		self.trailing_newline = ch == '\n';
	}

	/// Write indent spaces for nested list items.
	pub fn write_list_indent(&mut self) {
		if self.list_depth > 1 {
			for _ in 0..self.list_depth - 1 {
				self.push_raw("  ");
			}
		}
	}

	/// Handle entering a `<ul>` or `<ol>` element, updating list state.
	///
	/// Returns the list context that was pushed so callers can inspect it.
	pub fn enter_ul(&mut self) {
		if self.list_depth == 0 {
			self.ensure_block_separator();
		}
		self.list_depth += 1;
		self.list_stack.push(ListContext::Unordered);
	}

	pub fn enter_ol(&mut self, start: usize) {
		if self.list_depth == 0 {
			self.ensure_block_separator();
		}
		self.list_depth += 1;
		self.list_stack.push(ListContext::Ordered(start));
	}

	pub fn leave_list(&mut self) {
		self.list_depth = self.list_depth.saturating_sub(1);
		self.list_stack.pop();
	}

	/// Returns the next list item prefix (advancing ordered counters).
	///
	/// `unordered_bullet` is the marker used for unordered list items,
	/// eg `"- "` for markdown or `"• "` for ANSI output.
	pub fn next_list_prefix(&mut self, unordered_bullet: &str) -> String {
		match self.list_stack.last_mut() {
			Some(ListContext::Unordered) => unordered_bullet.to_string(),
			Some(ListContext::Ordered(num)) => {
				let prefix = format!("{}. ", num);
				*num += 1;
				prefix
			}
			None => unordered_bullet.to_string(),
		}
	}

	/// Returns the blockquote line prefix string for the current depth.
	///
	/// `prefix` is the per-level marker, eg `"> "` or `"▌ "`.
	pub fn blockquote_prefix(&self, marker: &str) -> String {
		marker.repeat(self.blockquote_depth)
	}

	/// Look up an attribute value by key from a visitor attrs slice.
	pub fn find_attr<'a>(
		attrs: &'a [(Entity, &Attribute, &Value)],
		key: &str,
	) -> Option<&'a Value> {
		attrs
			.iter()
			.find(|(_, attr, _)| attr.as_str() == key)
			.map(|(_, _, val)| *val)
	}

	/// Extract the `start` attribute as a `usize` for ordered lists.
	pub fn ol_start(attrs: &[(Entity, &Attribute, &Value)]) -> usize {
		Self::find_attr(attrs, "start")
			.and_then(|val| match val {
				Value::Uint(num) => Some(*num as usize),
				Value::Int(num) => Some(*num as usize),
				_ => None,
			})
			.unwrap_or(1)
	}
}
