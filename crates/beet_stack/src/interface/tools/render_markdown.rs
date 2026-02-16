//! Render semantic text content to markdown format.
//!
//! This module provides functionality to convert the semantic text representation
//! (using [`TextNode`] and semantic markers) into markdown strings.
//!
//! Supports all content types produced by [`MarkdownParser`](crate::parsers::MarkdownParser):
//!
//! - Block elements: headings, paragraphs, block quotes, code blocks,
//!   lists, tables, thematic breaks, images
//! - Inline markers: strong, emphasis, strikethrough, code, quote,
//!   superscript, subscript, links
//!
//! The core rendering logic is exposed via [`render_markdown_for`] so it can
//! be called from other systems (ie the interface tool) without needing
//! to go through a tool call.
//! ```
use crate::prelude::*;
use beet_core::prelude::*;


/// Creates a markdown rendering tool for an entity's text content tree.
///
/// This tool traverses the entity and its descendants within the card
/// boundary, converting semantic markers to their markdown equivalents:
///
/// - [`Heading1`]..=[`Heading6`] → `#`..=`######` (heading level)
/// - [`Paragraph`] → `text\n\n` (paragraph with trailing newlines)
/// - [`Important`] → `**text**` (bold)
/// - [`Emphasize`] → `*text*` (italic)
/// - [`Code`] → `` `text` `` (inline code)
/// - [`Quote`] → `"text"` (quoted)
/// - [`Link`] → `[text](url)`
/// - [`BlockQuote`] → `> text` (block quote)
/// - [`CodeBlock`] → fenced code block
/// - [`ListMarker`] + [`ListItem`] → `- item` or `1. item`
/// - [`ThematicBreak`] → `---`
/// - [`Image`] → `![alt](src)`
/// - [`Strikethrough`] → `~~text~~`
/// - [`Table`] → GFM table
/// - [`Superscript`] → `^text^`
/// - [`Subscript`] → `~text~`
///
/// # Returns
///
/// A bundle containing the markdown rendering tool that produces a markdown
/// string representing the text content of the entity tree.
///
/// # Example
///
/// ```ignore
/// use beet_stack::prelude::*;
/// use beet_core::prelude::*;
///
/// let val = World::new()
/// 			.spawn((render_markdown(), content!["hello world"]))
/// 			.send_blocking::<(), String>(())
/// 			.unwrap();
/// assert_eq!(val, "hello world");
///
pub fn render_markdown() -> impl Bundle {
	(
		PathPartial::new("render-markdown"),
		tool(render_markdown_system),
	)
}

/// Renders an entity's text content tree to markdown using direct world access.
///
/// This is the reusable entry point for markdown rendering. It runs the
/// rendering system via [`World::run_system_cached_with`], so it can be
/// called from any context that has `&mut World`.
pub fn render_markdown_for(entity: Entity, world: &mut World) -> String {
	world
		.run_system_cached_with(render_markdown_for_entity, entity)
		.unwrap_or_default()
}

/// System that renders an entity tree to markdown using CardQuery.
/// Used by the [`render_markdown`] tool, which renders relative to
/// its own entity (via card root resolution).
fn render_markdown_system(
	In(cx): In<ToolContext>,
	renderer: RenderMarkdown,
) -> Result<String> {
	renderer.render(cx.tool, true).xok()
}

/// System that renders a specific entity to markdown, starting from
/// that entity directly rather than resolving the card root first.
/// Used by [`render_markdown_for`].
fn render_markdown_for_entity(
	In(entity): In<Entity>,
	renderer: RenderMarkdown,
) -> String {
	renderer.render(entity, false)
}


/// System parameter that encapsulates all queries needed for
/// markdown rendering.
#[derive(SystemParam)]
pub struct RenderMarkdown<'w, 's> {
	card_query: CardQuery<'w, 's>,
	text_query: TextQuery<'w, 's>,
	text_content: Query<'w, 's, &'static TextNode>,
	important: Query<'w, 's, EntityRef<'static>, With<Important>>,
	emphasize: Query<'w, 's, (), With<Emphasize>>,
	code: Query<'w, 's, (), With<Code>>,
	quote: Query<'w, 's, (), With<Quote>>,
	links: Query<'w, 's, &'static Link>,
	ancestors: Query<'w, 's, &'static ChildOf>,
	cards: Query<'w, 's, (), With<Card>>,
	children_query: Query<'w, 's, &'static Children>,
	// Block-level queries
	block_quotes: Query<'w, 's, (), With<BlockQuote>>,
	code_blocks: Query<'w, 's, &'static CodeBlock>,
	list_markers: Query<'w, 's, &'static ListMarker>,
	list_items: Query<'w, 's, (), With<ListItem>>,
	thematic_breaks: Query<'w, 's, (), With<ThematicBreak>>,
	images: Query<'w, 's, &'static Image>,
	tables: Query<'w, 's, &'static Table>,
	table_heads: Query<'w, 's, (), With<TableHead>>,
	table_rows: Query<'w, 's, (), With<TableRow>>,
	table_cells: Query<'w, 's, (), With<TableCell>>,
	html_blocks: Query<'w, 's, &'static HtmlBlock>,
	math_displays: Query<'w, 's, (), With<MathDisplay>>,
	footnote_defs: Query<'w, 's, &'static FootnoteDefinition>,
	// Inline queries
	strikethrough: Query<'w, 's, (), With<Strikethrough>>,
	superscript: Query<'w, 's, (), With<Superscript>>,
	subscript: Query<'w, 's, (), With<Subscript>>,
	hard_breaks: Query<'w, 's, (), With<HardBreak>>,
	soft_breaks: Query<'w, 's, (), With<SoftBreak>>,
	math_inlines: Query<'w, 's, (), With<MathInline>>,
	html_inlines: Query<'w, 's, &'static HtmlInline>,
	footnote_refs: Query<'w, 's, &'static FootnoteRef>,
	task_checks: Query<'w, 's, &'static TaskListCheck>,
}

impl RenderMarkdown<'_, '_> {
	/// Render an entity's text tree to a markdown string.
	///
	/// When `resolve_card_root` is true, iteration starts from the
	/// card root of `entity`. When false, iteration starts directly
	/// from `entity`.
	fn render(&self, entity: Entity, resolve_card_root: bool) -> String {
		let root = if resolve_card_root {
			self.card_query.card_root(entity)
		} else {
			entity
		};
		let mut output = String::new();
		self.render_entity(root, &mut output, "");
		output
	}

	/// Recursively render an entity and its children to markdown.
	/// `prefix` is prepended to each line (used for block quote `> ` nesting).
	fn render_entity(&self, entity: Entity, out: &mut String, prefix: &str) {
		// -- Block-level elements --
		if self.text_query.is_heading(entity) {
			let level =
				self.text_query.heading_level(entity).unwrap_or(1) as usize;
			let hashes = "#".repeat(level.min(6));
			let inner = self.collect_inline_text(entity);
			if !inner.is_empty() {
				out.push_str(&format!("{prefix}{hashes} {inner}\n\n"));
			}
			return;
		}

		if self.text_query.is_structural(entity)
			&& !self.block_quotes.contains(entity)
			&& !self.list_items.contains(entity)
			&& !self.list_markers.contains(entity)
			&& !self.code_blocks.contains(entity)
			&& !self.tables.contains(entity)
			&& !self.table_heads.contains(entity)
			&& !self.table_rows.contains(entity)
			&& !self.table_cells.contains(entity)
			&& !self.thematic_breaks.contains(entity)
			&& !self.math_displays.contains(entity)
			&& !self.html_blocks.contains(entity)
			&& !self.footnote_defs.contains(entity)
		{
			// Paragraph or other generic structural block
			let inner = self.collect_inline_text(entity);
			if !inner.is_empty() {
				out.push_str(&format!("{prefix}{inner}\n\n"));
			}
			return;
		}

		if self.thematic_breaks.contains(entity) {
			out.push_str(&format!("{prefix}---\n\n"));
			return;
		}

		if self.block_quotes.contains(entity) {
			let new_prefix = format!("{prefix}> ");
			self.render_children(entity, out, &new_prefix);
			return;
		}

		if let Ok(code_block) = self.code_blocks.get(entity) {
			let lang = code_block.language.as_deref().unwrap_or("");
			out.push_str(&format!("{prefix}```{lang}\n"));
			// Code block content is stored as TextNode children
			if let Ok(children) = self.children_query.get(entity) {
				for child in children.iter() {
					if let Ok(text) = self.text_content.get(child) {
						for line in text.as_str().lines() {
							out.push_str(&format!("{prefix}{line}\n"));
						}
					}
				}
			}
			out.push_str(&format!("{prefix}```\n\n"));
			return;
		}

		if let Ok(list_marker) = self.list_markers.get(entity) {
			if let Ok(children) = self.children_query.get(entity) {
				for (idx, child) in children.iter().enumerate() {
					if self.list_items.contains(child) {
						let bullet = if list_marker.ordered {
							let start = list_marker.start.unwrap_or(1) as usize;
							format!("{}. ", start + idx)
						} else {
							"- ".to_string()
						};
						// Render task list checkbox if present
						let checkbox = self.render_task_checkbox(child);
						let inner = self.collect_list_item_text(child);
						out.push_str(&format!(
							"{prefix}{bullet}{checkbox}{inner}\n"
						));
					}
				}
			}
			out.push('\n');
			return;
		}

		if let Ok(table) = self.tables.get(entity) {
			self.render_table(entity, table, out, prefix);
			return;
		}

		if let Ok(image) = self.images.get(entity) {
			let alt = self.collect_inline_text(entity);
			let title = image
				.title
				.as_ref()
				.map(|title| format!(" \"{title}\""))
				.unwrap_or_default();
			out.push_str(&format!(
				"{prefix}![{alt}]({}{title})\n\n",
				image.src
			));
			return;
		}

		if let Ok(html_block) = self.html_blocks.get(entity) {
			if !html_block.0.is_empty() {
				out.push_str(&format!("{prefix}{}\n\n", html_block.0));
			}
			return;
		}

		if self.math_displays.contains(entity) {
			out.push_str(&format!("{prefix}$$\n"));
			if let Ok(children) = self.children_query.get(entity) {
				for child in children.iter() {
					if let Ok(text) = self.text_content.get(child) {
						out.push_str(&format!("{prefix}{}\n", text.as_str()));
					}
				}
			}
			out.push_str(&format!("{prefix}$$\n\n"));
			return;
		}

		if let Ok(footnote_def) = self.footnote_defs.get(entity) {
			out.push_str(&format!("{prefix}[^{}]: ", footnote_def.label));
			let inner = self.collect_inline_text_from_children(entity);
			out.push_str(&format!("{inner}\n\n"));
			return;
		}

		// -- Inline elements rendered at block level --
		if self.hard_breaks.contains(entity) {
			out.push_str("  \n");
			return;
		}
		if self.soft_breaks.contains(entity) {
			out.push('\n');
			return;
		}

		// Standalone inline text not inside a structural element
		if let Ok(text) = self.text_content.get(entity) {
			let parent_is_structural =
				self.ancestors.get(entity).is_ok_and(|child_of| {
					self.text_query.is_structural(child_of.parent())
				});
			if !parent_is_structural {
				out.push_str(&self.apply_inline_markers(text.as_str(), entity));
			}
			return;
		}

		// Generic container — recurse into children
		self.render_children(entity, out, prefix);
	}

	/// Render all children of an entity.
	fn render_children(&self, entity: Entity, out: &mut String, prefix: &str) {
		if let Ok(children) = self.children_query.get(entity) {
			for child in children.iter() {
				// Stop at card boundaries — skip nested Card entities
				if self.cards.contains(child) {
					continue;
				}
				self.render_entity(child, out, prefix);
			}
		}
	}

	/// Collect inline text from a structural element's children,
	/// applying inline markers to each segment. Skips text belonging
	/// to nested structural elements.
	fn collect_inline_text(&self, parent: Entity) -> String {
		let mut result = String::new();
		for child in self.card_query.iter_dfs_from(parent).skip(1) {
			// Skip structural elements and their descendants
			if self.text_query.is_structural(child) {
				continue;
			}
			// Skip text belonging to a different structural parent
			let text_parent =
				self.ancestors.get(child).map(|co| co.parent()).ok();
			if text_parent.is_some_and(|tp| {
				tp != parent && self.text_query.is_structural(tp)
			}) {
				continue;
			}
			if self.hard_breaks.contains(child) {
				result.push_str("  \n");
				continue;
			}
			if self.soft_breaks.contains(child) {
				result.push('\n');
				continue;
			}
			if let Ok(footnote_ref) = self.footnote_refs.get(child) {
				result.push_str(&format!("[^{}]", footnote_ref.label));
				continue;
			}
			if let Ok(html_inline) = self.html_inlines.get(child) {
				result.push_str(&html_inline.0);
				continue;
			}
			if let Ok(text) = self.text_content.get(child) {
				result
					.push_str(&self.apply_inline_markers(text.as_str(), child));
			}
		}
		result
	}

	/// Collect inline text from direct children only (no DFS).
	fn collect_inline_text_from_children(&self, parent: Entity) -> String {
		let mut result = String::new();
		if let Ok(children) = self.children_query.get(parent) {
			for child in children.iter() {
				if let Ok(text) = self.text_content.get(child) {
					result.push_str(
						&self.apply_inline_markers(text.as_str(), child),
					);
				}
			}
		}
		result
	}

	/// Collect text from a list item, including nested paragraphs.
	fn collect_list_item_text(&self, item: Entity) -> String {
		let mut result = String::new();
		if let Ok(children) = self.children_query.get(item) {
			for child in children.iter() {
				if self.task_checks.contains(child) {
					continue;
				}
				if self.text_query.is_structural(child) {
					let inner = self.collect_inline_text(child);
					if !result.is_empty() && !inner.is_empty() {
						result.push(' ');
					}
					result.push_str(&inner);
				} else if let Ok(text) = self.text_content.get(child) {
					result.push_str(
						&self.apply_inline_markers(text.as_str(), child),
					);
				}
			}
		}
		result
	}

	/// Render a task list checkbox prefix for a list item.
	fn render_task_checkbox(&self, item: Entity) -> String {
		if let Ok(children) = self.children_query.get(item) {
			for child in children.iter() {
				if let Ok(check) = self.task_checks.get(child) {
					return if check.checked {
						"[x] ".to_string()
					} else {
						"[ ] ".to_string()
					};
				}
			}
		}
		String::new()
	}

	/// Render a GFM-style markdown table.
	fn render_table(
		&self,
		entity: Entity,
		table: &Table,
		out: &mut String,
		prefix: &str,
	) {
		if let Ok(children) = self.children_query.get(entity) {
			for child in children.iter() {
				if self.table_heads.contains(child) {
					// Render header cells
					let row_text = self.render_table_row_cells(child);
					out.push_str(&format!("{prefix}| {} |\n", row_text));
					// Render separator
					let sep: Vec<String> = table
						.alignments
						.iter()
						.map(|alignment| match alignment {
							CellAlignment::Left => ":---".to_string(),
							CellAlignment::Center => ":---:".to_string(),
							CellAlignment::Right => "---:".to_string(),
							CellAlignment::None => "---".to_string(),
						})
						.collect();
					if sep.is_empty() {
						out.push_str(&format!("{prefix}| --- |\n"));
					} else {
						out.push_str(&format!(
							"{prefix}| {} |\n",
							sep.join(" | ")
						));
					}
				} else if self.table_rows.contains(child) {
					let row_text = self.render_table_row_cells(child);
					out.push_str(&format!("{prefix}| {} |\n", row_text));
				}
			}
		}
		out.push('\n');
	}

	/// Render cells of a table row as `cell1 | cell2 | ...`.
	fn render_table_row_cells(&self, row: Entity) -> String {
		let mut cells = Vec::new();
		if let Ok(children) = self.children_query.get(row) {
			for child in children.iter() {
				if self.table_cells.contains(child) {
					cells.push(self.collect_inline_text(child));
				}
			}
		}
		cells.join(" | ")
	}

	/// Apply inline markers (code, emphasis, importance, quote, link,
	/// strikethrough, superscript, subscript, math) to a text string.
	fn apply_inline_markers(&self, text: &str, entity: Entity) -> String {
		let mut wrapped = text.to_string();

		if self.math_inlines.contains(entity) {
			wrapped = format!("${wrapped}$");
		}
		if self.code.contains(entity) {
			wrapped = format!("`{wrapped}`");
		}
		if self.subscript.contains(entity) {
			wrapped = format!("~{wrapped}~");
		}
		if self.superscript.contains(entity) {
			wrapped = format!("^{wrapped}^");
		}
		if self.strikethrough.contains(entity) {
			wrapped = format!("~~{wrapped}~~");
		}
		if self.emphasize.contains(entity) {
			wrapped = format!("*{wrapped}*");
		}
		if self.important.contains(entity) {
			wrapped = format!("**{wrapped}**");
		}
		if self.quote.contains(entity) {
			wrapped = format!("\"{wrapped}\"");
		}
		if let Ok(link) = self.links.get(entity) {
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


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn plain_text() {
		World::new()
			.spawn((render_markdown(), content!["hello world"]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("hello world");
	}

	#[test]
	fn multiple_segments() {
		World::new()
			.spawn((render_markdown(), content!["hello", " ", "world"]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("hello world");
	}

	#[test]
	fn important_text() {
		World::new()
			.spawn((render_markdown(), content![
				"hello ",
				(Important, "bold"),
				" text"
			]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("hello **bold** text");
	}

	#[test]
	fn emphasized_text() {
		World::new()
			.spawn((render_markdown(), content![
				"hello ",
				(Emphasize, "italic"),
				" text"
			]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("hello *italic* text");
	}

	#[test]
	fn code_text() {
		World::new()
			.spawn((render_markdown(), content![
				"use ",
				(Code, "println!"),
				" macro"
			]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("use `println!` macro");
	}

	#[test]
	fn quoted_text() {
		World::new()
			.spawn((render_markdown(), content!["he said ", (Quote, "hello")]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("he said \"hello\"");
	}

	#[test]
	fn link_without_title() {
		World::new()
			.spawn((render_markdown(), children![(
				TextNode::new("click here"),
				Link::new("https://example.com")
			)]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("[click here](https://example.com)");
	}

	#[test]
	fn link_with_title() {
		World::new()
			.spawn((render_markdown(), children![(
				TextNode::new("example"),
				Link::new("https://example.com").with_title("Example Site")
			)]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("[example](https://example.com \"Example Site\")");
	}


	#[test]
	fn combined_markers() {
		World::new()
			.spawn((render_markdown(), content![(
				Important,
				Emphasize,
				"bold italic"
			)]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("***bold italic***");
	}

	#[test]
	fn complex_composition() {
		World::new()
			.spawn((render_markdown(), content![
				"Welcome to ",
				(Important, "beet"),
				", the ",
				(Emphasize, "best"),
				" framework!"
			]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("Welcome to **beet**, the *best* framework!");
	}

	#[test]
	fn extension_trait() {
		World::new()
			.spawn((render_markdown(), content!["test"]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("test");
	}

	#[test]
	fn important_link() {
		World::new()
			.spawn((render_markdown(), children![(
				Important,
				TextNode::new("important link"),
				Link::new("https://example.com")
			)]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("[**important link**](https://example.com)");
	}

	#[test]
	fn all_markers_combined() {
		World::new()
			.spawn((render_markdown(), children![(
				Important,
				Emphasize,
				Code,
				Quote,
				TextNode::new("text"),
				Link::new("https://example.com")
			)]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("[\"***`text`***\"](https://example.com)");
	}

	#[test]
	fn heading_renders_as_heading() {
		World::new()
			.spawn((render_markdown(), Heading1::with_text("Hello World")))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("# Hello World\n\n");
	}

	#[test]
	fn heading2_renders_as_h2() {
		World::new()
			.spawn((render_markdown(), children![
				Heading1::with_text("Outer"),
				Heading2::with_text("Inner"),
			]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("# Outer\n\n## Inner\n\n");
	}

	#[test]
	fn paragraph_renders_with_newlines() {
		World::new()
			.spawn((
				render_markdown(),
				Paragraph::with_text("A paragraph of text."),
			))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("A paragraph of text.\n\n");
	}

	#[test]
	fn mixed_structure() {
		World::new()
			.spawn((render_markdown(), children![
				Heading1::with_text("Welcome"),
				Paragraph::with_text("This is the intro.")
			]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("# Welcome\n\nThis is the intro.\n\n");
	}

	#[test]
	fn respects_card_boundary() {
		World::new()
			.spawn((render_markdown(), Card, children![
				Paragraph::with_text("Inside card"),
				// Nested card should not be rendered
				(Card, children![Paragraph::with_text("Inside nested card")])
			]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("Inside card\n\n");
	}

	#[test]
	fn render_markdown_for_works() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, content!["hello ", (Important, "world")]))
			.id();

		let result = render_markdown_for(entity, &mut world);
		result.xpect_eq("hello **world**");
	}

	#[test]
	fn render_markdown_for_respects_card_boundary() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![
				Paragraph::with_text("visible"),
				(Card, children![Paragraph::with_text("hidden")])
			]))
			.id();

		let result = render_markdown_for(entity, &mut world);
		result.xpect_eq("visible\n\n");
	}
}
