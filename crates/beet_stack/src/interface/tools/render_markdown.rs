//! Render semantic text content to markdown format.
//!
//! This module provides functionality to convert the semantic text representation
//! (using [`TextContent`] and semantic markers) into markdown strings.
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
/// - [`Title`] → `# text` (heading level based on nesting)
/// - [`Paragraph`] → `text\n\n` (paragraph with trailing newlines)
/// - [`Important`] → `**text**` (bold)
/// - [`Emphasize`] → `*text*` (italic)
/// - [`Code`] → `` `text` `` (inline code)
/// - [`Quote`] → `"text"` (quoted)
/// - [`Link`] → `[text](url)`
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
	text_content: Query<'w, 's, &'static TextContent>,
	important: Query<'w, 's, (), With<Important>>,
	emphasize: Query<'w, 's, (), With<Emphasize>>,
	code: Query<'w, 's, (), With<Code>>,
	quote: Query<'w, 's, (), With<Quote>>,
	links: Query<'w, 's, &'static Link>,
	ancestors: Query<'w, 's, &'static ChildOf>,
}

impl RenderMarkdown<'_, '_> {
	/// Render an entity's text tree to a markdown string.
	///
	/// When `resolve_card_root` is true, iteration starts from the
	/// card root of `entity`. When false, iteration starts directly
	/// from `entity`.
	fn render(&self, entity: Entity, resolve_card_root: bool) -> String {
		let mut output = String::new();

		let iter: Box<dyn Iterator<Item = Entity>> = if resolve_card_root {
			Box::new(self.card_query.iter_dfs(entity))
		} else {
			Box::new(self.card_query.iter_dfs_from(entity))
		};

		for current in iter {
			let is_title = self.text_query.is_title(current);
			let is_paragraph =
				!is_title && self.text_query.is_structural(current);

			if is_title || is_paragraph {
				let inner_text = self.collect_inline_text(current);
				if !inner_text.is_empty() {
					if is_title {
						let level =
							self.text_query.title_level(current) as usize + 1;
						let hashes = "#".repeat(level.min(6));
						output.push_str(&format!(
							"{} {}\n\n",
							hashes, inner_text
						));
					} else {
						output.push_str(&format!("{}\n\n", inner_text));
					}
				}
				continue;
			}

			// Standalone inline text not inside a structural element
			if let Ok(text) = self.text_content.get(current) {
				let parent_is_structural =
					self.ancestors.get(current).is_ok_and(|child_of| {
						self.text_query.is_structural(child_of.parent())
					});
				if !parent_is_structural {
					output.push_str(
						&self.apply_inline_markers(text.as_str(), current),
					);
				}
			}
		}

		output
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
			if let Ok(text) = self.text_content.get(child) {
				result
					.push_str(&self.apply_inline_markers(text.as_str(), child));
			}
		}
		result
	}

	/// Apply inline markers (code, emphasis, importance, quote, link)
	/// to a text string.
	fn apply_inline_markers(&self, text: &str, entity: Entity) -> String {
		let mut wrapped = text.to_string();

		if self.code.contains(entity) {
			wrapped = format!("`{}`", wrapped);
		}
		if self.emphasize.contains(entity) {
			wrapped = format!("*{}*", wrapped);
		}
		if self.important.contains(entity) {
			wrapped = format!("**{}**", wrapped);
		}
		if self.quote.contains(entity) {
			wrapped = format!("\"{}\"", wrapped);
		}
		if let Ok(link) = self.links.get(entity) {
			let title = link
				.title
				.as_ref()
				.map(|title| format!(" \"{}\"", title))
				.unwrap_or_default();
			wrapped = format!("[{}]({}{})", wrapped, link.href, title);
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
				TextContent::new("click here"),
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
				TextContent::new("example"),
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
				TextContent::new("important link"),
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
				TextContent::new("text"),
				Link::new("https://example.com")
			)]))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("[\"***`text`***\"](https://example.com)");
	}

	#[test]
	fn title_renders_as_heading() {
		World::new()
			.spawn((render_markdown(), Title::with_text("Hello World")))
			.call_blocking::<(), String>(())
			.unwrap()
			.xpect_eq("# Hello World\n\n");
	}

	#[test]
	fn nested_title_increments_level() {
		World::new()
			.spawn((render_markdown(), Title, children![
				TextContent::new("Outer"),
				Title::with_text("Inner"),
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
				Title::with_text("Welcome"),
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
