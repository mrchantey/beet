//! Render content trees to markdown via the [`CardVisitor`] pattern.
//!
//! [`MarkdownRenderer`] walks a card's entity tree and produces a
//! markdown string. It handles all block-level and inline elements,
//! including nested block quotes, lists, tables, and inline
//! formatting markers via [`InlineStyle`].
//!
//! # Usage
//!
//! ```ignore
//! use beet_stack::prelude::*;
//! use beet_core::prelude::*;
//!
//! fn render(walker: CardWalker, entity: Entity) -> String {
//!     let mut renderer = MarkdownRenderer::new();
//!     walker.walk_card(&mut renderer, entity);
//!     renderer.finish()
//! }
//! ```
use crate::prelude::*;
use beet_core::prelude::*;
use std::ops::ControlFlow;

/// Visitor-based markdown renderer.
///
/// Accumulates markdown text as the [`CardWalker`] traverses the
/// content tree. Call [`finish`](Self::finish) after walking to
/// retrieve the final markdown string.
pub struct MarkdownRenderer {
	/// Accumulated output.
	buffer: String,
	/// Stack of line prefixes for nested block quotes.
	prefix_stack: Vec<String>,
	/// Current list context stack: `(ordered, start, current_index)`.
	list_stack: Vec<ListContext>,
	/// Whether we are inside a code block (suppress inline formatting).
	in_code_block: bool,
	/// Language tag for the current code block.
	code_block_lang: Option<String>,
	/// Tracks table column alignments for the current table.
	table_alignments: Vec<TextAlignment>,
	/// Collects cells for the current table row.
	table_row_cells: Vec<String>,
	/// Whether we are in a table head (to emit separator after).
	in_table_head: bool,
	/// Whether we are currently inside a table cell (collecting text).
	in_table_cell: bool,
	/// Whether we are inside a list item.
	in_list_item: bool,
	/// Buffer for collecting list item text.
	list_item_buffer: String,
	/// Whether the current list item had a task checkbox rendered.
	list_item_had_checkbox: bool,
	/// Buffer for collecting footnote definition text.
	footnote_buffer: Option<String>,
}

struct ListContext {
	ordered: bool,
	start: u64,
	current_index: u64,
}

impl MarkdownRenderer {
	/// Create a new empty renderer.
	pub fn new() -> Self {
		Self {
			buffer: String::new(),
			prefix_stack: Vec::new(),
			list_stack: Vec::new(),
			in_code_block: false,
			code_block_lang: None,
			table_alignments: Vec::new(),
			table_row_cells: Vec::new(),
			in_table_head: false,
			in_table_cell: false,
			in_list_item: false,
			list_item_buffer: String::new(),
			list_item_had_checkbox: false,
			footnote_buffer: None,
		}
	}

	/// Consume the renderer and return the accumulated markdown.
	pub fn finish(self) -> String { self.buffer }

	/// Current line prefix (from nested block quotes).
	fn prefix(&self) -> String {
		self.prefix_stack.last().cloned().unwrap_or_default()
	}

	/// Push text to the active buffer (cell, list item, footnote,
	/// or main buffer).
	fn push_text(&mut self, text: &str) {
		if self.in_table_cell {
			if let Some(cell) = self.table_row_cells.last_mut() {
				cell.push_str(text);
			}
		} else if self.in_list_item {
			self.list_item_buffer.push_str(text);
		} else if let Some(ref mut fb) = self.footnote_buffer {
			fb.push_str(text);
		} else {
			self.buffer.push_str(text);
		}
	}

	/// Apply inline formatting markers to a text string and return
	/// the wrapped result.
	fn apply_inline_style(&self, text: &str, style: &InlineStyle) -> String {
		let mut wrapped = text.to_string();

		if style.math_inline {
			wrapped = format!("${wrapped}$");
		}
		if style.code {
			wrapped = format!("`{wrapped}`");
		}
		if style.subscript {
			wrapped = format!("~{wrapped}~");
		}
		if style.superscript {
			wrapped = format!("^{wrapped}^");
		}
		if style.strikethrough {
			wrapped = format!("~~{wrapped}~~");
		}
		if style.emphasize {
			wrapped = format!("*{wrapped}*");
		}
		if style.important {
			wrapped = format!("**{wrapped}**");
		}
		if style.quote {
			wrapped = format!("\"{wrapped}\"");
		}
		if let Some(ref link) = style.link {
			let title = link
				.title
				.as_ref()
				.map(|title| format!(" \"{title}\""))
				.unwrap_or_default();
			wrapped = format!("[{wrapped}]({}{title})", link.href);
		}
		wrapped
	}
}

impl Default for MarkdownRenderer {
	fn default() -> Self { Self::new() }
}

impl CardVisitor for MarkdownRenderer {
	// -- Block-level visit --

	fn visit_heading(
		&mut self,
		_entity: Entity,
		heading: &Heading,
	) -> ControlFlow<()> {
		let level = heading.level() as usize;
		let hashes = "#".repeat(level.min(6));
		let prefix = self.prefix();
		self.push_text(&format!("{prefix}{hashes} "));
		ControlFlow::Continue(())
	}

	fn visit_paragraph(&mut self, _entity: Entity) -> ControlFlow<()> {
		// Text will be accumulated via visit_text; we just mark the start
		ControlFlow::Continue(())
	}

	fn visit_block_quote(&mut self, _entity: Entity) -> ControlFlow<()> {
		let new_prefix = format!("{}> ", self.prefix());
		self.prefix_stack.push(new_prefix);
		ControlFlow::Continue(())
	}

	fn visit_code_block(
		&mut self,
		_entity: Entity,
		code_block: &CodeBlock,
	) -> ControlFlow<()> {
		let prefix = self.prefix();
		let lang = code_block.language.as_deref().unwrap_or("");
		self.push_text(&format!("{prefix}```{lang}\n"));
		self.in_code_block = true;
		self.code_block_lang = code_block.language.clone();
		ControlFlow::Continue(())
	}

	fn visit_list(
		&mut self,
		_entity: Entity,
		list_marker: &ListMarker,
	) -> ControlFlow<()> {
		self.list_stack.push(ListContext {
			ordered: list_marker.ordered,
			start: list_marker.start.unwrap_or(1),
			current_index: 0,
		});
		ControlFlow::Continue(())
	}

	fn visit_list_item(&mut self, _entity: Entity) -> ControlFlow<()> {
		self.in_list_item = true;
		self.list_item_buffer.clear();
		self.list_item_had_checkbox = false;
		ControlFlow::Continue(())
	}

	fn visit_table(
		&mut self,
		_entity: Entity,
		table: &Table,
	) -> ControlFlow<()> {
		self.table_alignments = table.alignments.clone();
		ControlFlow::Continue(())
	}

	fn visit_table_head(&mut self, _entity: Entity) -> ControlFlow<()> {
		self.in_table_head = true;
		self.table_row_cells.clear();
		ControlFlow::Continue(())
	}

	fn visit_table_row(&mut self, _entity: Entity) -> ControlFlow<()> {
		self.table_row_cells.clear();
		ControlFlow::Continue(())
	}

	fn visit_table_cell(&mut self, _entity: Entity) -> ControlFlow<()> {
		self.in_table_cell = true;
		self.table_row_cells.push(String::new());
		ControlFlow::Continue(())
	}

	fn visit_thematic_break(&mut self, _entity: Entity) -> ControlFlow<()> {
		let prefix = self.prefix();
		self.push_text(&format!("{prefix}---\n\n"));
		ControlFlow::Continue(())
	}

	fn visit_image(
		&mut self,
		_entity: Entity,
		image: &Image,
	) -> ControlFlow<()> {
		let prefix = self.prefix();
		let title = image
			.title
			.as_ref()
			.map(|title| format!(" \"{title}\""))
			.unwrap_or_default();
		// Render the full image tag and skip children. Alt text from
		// TextNode children is not yet collected here.
		self.push_text(&format!("{prefix}![]({}{})\n\n", image.src, title));
		ControlFlow::Break(())
	}

	fn visit_footnote_definition(
		&mut self,
		_entity: Entity,
		footnote_def: &FootnoteDefinition,
	) -> ControlFlow<()> {
		let prefix = self.prefix();
		self.push_text(&format!("{prefix}[^{}]: ", footnote_def.label));
		self.footnote_buffer = Some(String::new());
		ControlFlow::Continue(())
	}

	fn visit_math_display(&mut self, _entity: Entity) -> ControlFlow<()> {
		let prefix = self.prefix();
		self.push_text(&format!("{prefix}$$\n"));
		self.in_code_block = true; // reuse flag to suppress inline formatting
		ControlFlow::Continue(())
	}

	fn visit_html_block(
		&mut self,
		_entity: Entity,
		html_block: &HtmlBlock,
	) -> ControlFlow<()> {
		if !html_block.0.is_empty() {
			let prefix = self.prefix();
			self.push_text(&format!("{prefix}{}\n\n", html_block.0));
		}
		ControlFlow::Break(())
	}

	fn visit_button(
		&mut self,
		_entity: Entity,
		label: Option<&TextNode>,
	) -> ControlFlow<()> {
		// Render buttons as bold text in markdown
		if let Some(text) = label {
			let prefix = self.prefix();
			self.push_text(&format!("{prefix}**[{}]**", text.as_str()));
		}
		ControlFlow::Break(())
	}

	// -- Inline --

	fn visit_text(
		&mut self,
		_entity: Entity,
		text: &TextNode,
		style: &InlineStyle,
	) -> ControlFlow<()> {
		if self.in_code_block {
			let prefix = self.prefix();
			for line in text.as_str().lines() {
				self.push_text(&format!("{prefix}{line}\n"));
			}
		} else {
			let formatted = self.apply_inline_style(text.as_str(), style);
			self.push_text(&formatted);
		}
		ControlFlow::Continue(())
	}

	fn visit_hard_break(&mut self, _entity: Entity) -> ControlFlow<()> {
		self.push_text("  \n");
		ControlFlow::Continue(())
	}

	fn visit_soft_break(&mut self, _entity: Entity) -> ControlFlow<()> {
		self.push_text("\n");
		ControlFlow::Continue(())
	}

	fn visit_footnote_ref(
		&mut self,
		_entity: Entity,
		footnote_ref: &FootnoteRef,
	) -> ControlFlow<()> {
		self.push_text(&format!("[^{}]", footnote_ref.label));
		ControlFlow::Continue(())
	}

	fn visit_html_inline(
		&mut self,
		_entity: Entity,
		html_inline: &HtmlInline,
	) -> ControlFlow<()> {
		self.push_text(&html_inline.0);
		ControlFlow::Continue(())
	}

	fn visit_task_list_check(
		&mut self,
		_entity: Entity,
		task_check: &TaskListCheck,
	) -> ControlFlow<()> {
		if task_check.checked {
			self.push_text("[x] ");
		} else {
			self.push_text("[ ] ");
		}
		self.list_item_had_checkbox = true;
		ControlFlow::Continue(())
	}

	// -- Block-level leave --

	fn leave_heading(&mut self, _entity: Entity) { self.push_text("\n\n"); }

	fn leave_paragraph(&mut self, _entity: Entity) { self.push_text("\n\n"); }

	fn leave_block_quote(&mut self, _entity: Entity) {
		self.prefix_stack.pop();
	}

	fn leave_code_block(&mut self, _entity: Entity) {
		let prefix = self.prefix();
		self.in_code_block = false;
		self.code_block_lang = None;
		self.push_text(&format!("{prefix}```\n\n"));
	}

	fn leave_list(&mut self, _entity: Entity) {
		self.list_stack.pop();
		self.push_text("\n");
	}

	fn leave_list_item(&mut self, _entity: Entity) {
		self.in_list_item = false;
		let prefix = self.prefix();

		let bullet = if let Some(ctx) = self.list_stack.last_mut() {
			let bullet = if ctx.ordered {
				let num = ctx.start + ctx.current_index;
				format!("{num}. ")
			} else {
				"- ".to_string()
			};
			ctx.current_index += 1;
			bullet
		} else {
			"- ".to_string()
		};

		let text = std::mem::take(&mut self.list_item_buffer);
		self.buffer.push_str(&format!("{prefix}{bullet}{text}\n"));
	}

	fn leave_table(&mut self, _entity: Entity) {
		self.table_alignments.clear();
		self.push_text("\n");
	}

	fn leave_table_head(&mut self, _entity: Entity) {
		// Emit header row
		let prefix = self.prefix();
		let row_text = self.table_row_cells.join(" | ");
		self.buffer.push_str(&format!("{prefix}| {row_text} |\n"));

		// Emit separator
		let sep: Vec<String> = self
			.table_alignments
			.iter()
			.map(|alignment| match alignment {
				TextAlignment::Left => ":---".to_string(),
				TextAlignment::Center => ":---:".to_string(),
				TextAlignment::Right => "---:".to_string(),
				TextAlignment::None => "---".to_string(),
			})
			.collect();
		if sep.is_empty() {
			// Fallback: one separator per cell
			let fallback: Vec<&str> =
				vec!["---"; self.table_row_cells.len().max(1)];
			self.buffer
				.push_str(&format!("{prefix}| {} |\n", fallback.join(" | ")));
		} else {
			self.buffer
				.push_str(&format!("{prefix}| {} |\n", sep.join(" | ")));
		}

		self.in_table_head = false;
		self.table_row_cells.clear();
	}

	fn leave_table_row(&mut self, _entity: Entity) {
		let prefix = self.prefix();
		let row_text = self.table_row_cells.join(" | ");
		self.buffer.push_str(&format!("{prefix}| {row_text} |\n"));
		self.table_row_cells.clear();
	}

	fn leave_table_cell(&mut self, _entity: Entity) {
		self.in_table_cell = false;
	}
}


#[cfg(test)]
mod test {
	use super::*;

	/// Helper: spawn a card, walk it with `MarkdownRenderer`, return
	/// the markdown string.
	fn render_card(world: &mut World, entity: Entity) -> String {
		world
			.run_system_once_with(
				|In(entity): In<Entity>, walker: CardWalker| {
					let mut renderer = MarkdownRenderer::new();
					walker.walk_card(&mut renderer, entity);
					renderer.finish()
				},
				entity,
			)
			.unwrap()
	}

	// -- Plain text --

	#[test]
	fn plain_text() {
		let mut world = World::new();
		let entity = world.spawn((Card, content!["hello world"])).id();
		render_card(&mut world, entity).xpect_eq("hello world");
	}

	#[test]
	fn multiple_segments() {
		let mut world = World::new();
		let entity = world.spawn((Card, content!["hello", " ", "world"])).id();
		render_card(&mut world, entity).xpect_eq("hello world");
	}

	// -- Inline formatting --

	#[test]
	fn important_text() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, content!["hello ", (Important, "bold"), " text"]))
			.id();
		render_card(&mut world, entity).xpect_eq("hello **bold** text");
	}

	#[test]
	fn emphasized_text() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, content!["hello ", (Emphasize, "italic"), " text"]))
			.id();
		render_card(&mut world, entity).xpect_eq("hello *italic* text");
	}

	#[test]
	fn code_text() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, content!["use ", (Code, "println!"), " macro"]))
			.id();
		render_card(&mut world, entity).xpect_eq("use `println!` macro");
	}

	#[test]
	fn quoted_text() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, content!["he said ", (Quote, "hello")]))
			.id();
		render_card(&mut world, entity).xpect_eq("he said \"hello\"");
	}

	#[test]
	fn combined_markers() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, content![(Important, Emphasize, "bold italic")]))
			.id();
		render_card(&mut world, entity).xpect_eq("***bold italic***");
	}

	#[test]
	fn complex_composition() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, content![
				"Welcome to ",
				(Important, "beet"),
				", the ",
				(Emphasize, "best"),
				" framework!"
			]))
			.id();
		render_card(&mut world, entity)
			.xpect_eq("Welcome to **beet**, the *best* framework!");
	}

	// -- Links --

	#[test]
	fn link_without_title() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![(
				TextNode::new("click here"),
				Link::new("https://example.com"),
			)]))
			.id();
		render_card(&mut world, entity)
			.xpect_eq("[click here](https://example.com)");
	}

	#[test]
	fn link_with_title() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![(
				TextNode::new("example"),
				Link::new("https://example.com").with_title("Example Site"),
			)]))
			.id();
		render_card(&mut world, entity)
			.xpect_eq("[example](https://example.com \"Example Site\")");
	}

	#[test]
	fn important_link() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![(
				Important,
				TextNode::new("important link"),
				Link::new("https://example.com"),
			)]))
			.id();
		render_card(&mut world, entity)
			.xpect_eq("[**important link**](https://example.com)");
	}

	#[test]
	fn all_markers_combined() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![(
				Important,
				Emphasize,
				Code,
				Quote,
				TextNode::new("text"),
				Link::new("https://example.com"),
			)]))
			.id();
		render_card(&mut world, entity)
			.xpect_eq("[\"***`text`***\"](https://example.com)");
	}

	// -- Structural elements --

	#[test]
	fn heading_renders() {
		let mut world = World::new();
		let entity =
			world.spawn((Card, Heading1::with_text("Hello World"))).id();
		render_card(&mut world, entity).xpect_eq("# Hello World\n\n");
	}

	#[test]
	fn heading2_renders() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![
				Heading1::with_text("Outer"),
				Heading2::with_text("Inner"),
			]))
			.id();
		render_card(&mut world, entity).xpect_eq("# Outer\n\n## Inner\n\n");
	}

	#[test]
	fn paragraph_renders() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, Paragraph::with_text("A paragraph of text.")))
			.id();
		render_card(&mut world, entity).xpect_eq("A paragraph of text.\n\n");
	}

	#[test]
	fn mixed_structure() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![
				Heading1::with_text("Welcome"),
				Paragraph::with_text("This is the intro."),
			]))
			.id();
		render_card(&mut world, entity)
			.xpect_eq("# Welcome\n\nThis is the intro.\n\n");
	}

	// -- Card boundaries --

	#[test]
	fn respects_card_boundary() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![
				Paragraph::with_text("Inside card"),
				(Card, children![Paragraph::with_text("Inside nested card")]),
			]))
			.id();
		render_card(&mut world, entity).xpect_eq("Inside card\n\n");
	}

	// -- Thematic break --

	#[test]
	fn thematic_break() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![
				Paragraph::with_text("above"),
				(ThematicBreak,),
				Paragraph::with_text("below"),
			]))
			.id();
		render_card(&mut world, entity).xpect_eq("above\n\n---\n\nbelow\n\n");
	}

	// -- Container-style inline markers (markdown parser pattern) --

	#[test]
	fn important_container() {
		// Mimics the markdown parser structure where Important is a
		// container entity with TextNode children.
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![(Paragraph, children![
				TextNode::new("hello "),
				(Important, children![TextNode::new("bold")]),
				TextNode::new(" text"),
			])]))
			.id();
		render_card(&mut world, entity).xpect_eq("hello **bold** text\n\n");
	}

	#[test]
	fn nested_containers() {
		// Important wrapping Emphasize, both as containers.
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![(Paragraph, children![(
				Important,
				children![(Emphasize, children![TextNode::new("bold italic")])],
			)])]))
			.id();
		render_card(&mut world, entity).xpect_eq("***bold italic***\n\n");
	}

	#[test]
	fn link_container() {
		// Link as a container with TextNode child (parser pattern).
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![(Paragraph, children![(
				Link::new("https://example.com"),
				children![TextNode::new("click here")],
			)])]))
			.id();
		render_card(&mut world, entity)
			.xpect_eq("[click here](https://example.com)\n\n");
	}

	#[test]
	fn important_link_container() {
		// Important wrapping a Link container.
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![(Paragraph, children![(
				Important,
				children![(Link::new("https://example.com"), children![
					TextNode::new("important link")
				],)],
			)])]))
			.id();
		render_card(&mut world, entity)
			.xpect_eq("[**important link**](https://example.com)\n\n");
	}
}
