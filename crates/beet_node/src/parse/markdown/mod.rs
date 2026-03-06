//! Markdown parser that produces the same ECS entity tree as [`super::html`].
//!
//! Uses `pulldown-cmark` to tokenize markdown, converts events into the shared
//! [`TreeNode`](super::html::diff::TreeNode) intermediate representation, then
//! diffs against entities using the same infrastructure as the HTML parser.
//!
//! Embedded HTML blocks and inline HTML are delegated to the HTML tokenizer.
//!
//! Enable with the `markdown_parser` feature flag.

mod frontmatter;
pub(crate) mod tree_builder;

pub use frontmatter::*;

use crate::prelude::*;
use beet_core::prelude::*;
use pulldown_cmark::Options;

/// A configurable markdown parser that implements [`NodeParser`].
///
/// Parses markdown into a tree of ECS entities using the same [`TreeNode`]
/// and diff infrastructure as [`HtmlParser`]. Embedded HTML is delegated
/// to the HTML tokenizer via an internal [`HtmlParser`] configuration.
///
/// ## Example
/// ```rust
/// # use beet_node::prelude::*;
/// let parser = MarkdownParser::new();
/// ```
#[derive(Debug, Clone)]
pub struct MarkdownParser {
	/// HTML parser config used for embedded HTML blocks and inline HTML.
	pub html: HtmlParser,
	/// pulldown-cmark options controlling which extensions are enabled.
	pub options: Options,
	/// Whether to parse frontmatter metadata blocks.
	pub parse_frontmatter: bool,
}

impl Default for MarkdownParser {
	fn default() -> Self {
		Self {
			html: HtmlParser::default(),
			options: Self::default_options(),
			parse_frontmatter: true,
		}
	}
}

impl MarkdownParser {
	/// Create a new parser with default settings and maximal extensions.
	pub fn new() -> Self { Self::default() }

	/// Create a parser with expression support enabled in embedded HTML.
	pub fn with_expressions() -> Self {
		Self {
			html: HtmlParser::with_expressions(),
			..Default::default()
		}
	}

	/// Returns the default pulldown-cmark options with maximal extensions.
	pub fn default_options() -> Options {
		Options::ENABLE_TABLES
			| Options::ENABLE_FOOTNOTES
			| Options::ENABLE_STRIKETHROUGH
			| Options::ENABLE_TASKLISTS
			| Options::ENABLE_HEADING_ATTRIBUTES
			| Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
			| Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS
			| Options::ENABLE_MATH
			| Options::ENABLE_GFM
			| Options::ENABLE_DEFINITION_LIST
			| Options::ENABLE_SUPERSCRIPT
			| Options::ENABLE_SUBSCRIPT
	}

	/// Shared parsing logic: tokenize markdown, build tree, diff against entity.
	async fn parse_text(
		&self,
		entity: &AsyncEntity,
		text: &str,
		path: Option<&WsPathBuf>,
	) -> Result {
		let options = self.options;
		let html_parse_config = self.html.parse_config.clone();
		let html_diff_config = self.html.diff_config.clone();
		let parse_frontmatter = self.parse_frontmatter;
		let entity_id = entity.id();
		let text_owned = text.to_string();
		let path_owned = path.cloned();

		entity
			.world()
			.with_then(move |world| -> Result {
				let span_lookup = path_owned
					.as_ref()
					.map(|path| SpanLookup::new(&text_owned, path.clone()));

				let tree_result = tree_builder::build_markdown_tree(
					&text_owned,
					options,
					&html_parse_config,
					&html_diff_config,
					span_lookup.as_ref(),
				)?;

				// diff tree against entity
				diff_children(
					world,
					entity_id,
					&tree_result.nodes,
					&html_diff_config,
					span_lookup.as_ref(),
				)?;

				// insert frontmatter on root if present
				if parse_frontmatter {
					if let Some(fm) = tree_result.frontmatter {
						world.entity_mut(entity_id).insert(fm);
					} else {
						// remove stale frontmatter if content no longer has it
						world.entity_mut(entity_id).remove::<Frontmatter>();
					}
				}

				// insert file span on the root entity if path provided
				if let Some(ref lookup) = span_lookup {
					let span = lookup.full_span();
					world.entity_mut(entity_id).set_if_ne_or_insert(span);
				}

				Ok(())
			})
			.await?;

		Ok(())
	}
}

impl NodeParser for MarkdownParser {
	fn parse(
		&mut self,
		entity: AsyncEntity,
		bytes: Vec<u8>,
		path: Option<WsPathBuf>,
	) -> impl Future<Output = Result> {
		async move {
			let text = std::str::from_utf8(&bytes)?;
			self.parse_text(&entity, text, path.as_ref()).await
		}
	}
}

// Re-export diff internals used by tree_builder within crate
use crate::parse::html::diff::diff_children;


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Collect the entity ids of the direct children via [`AsyncEntity`].
	async fn get_children(entity: &AsyncEntity) -> Vec<Entity> {
		entity
			.with_then(|entity| {
				entity
					.get::<Children>()
					.map(|children| {
						let mut ids = Vec::new();
						for &child in children {
							ids.push(child);
						}
						ids
					})
					.unwrap_or_default()
			})
			.await
	}

	#[beet_core::test]
	async fn parse_simple_paragraph() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				MarkdownParser::new()
					.parse(entity, b"Hello world".to_vec(), None)
					.await
					.unwrap();
				let children = get_children(&entity).await;
				// should have one child: the <p> element
				children.len()
			})
			.await
			.xpect_eq(1);
	}

	#[beet_core::test]
	async fn parse_paragraph_contains_text() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				MarkdownParser::new()
					.parse(entity, b"Hello world".to_vec(), None)
					.await
					.unwrap();
				let children = get_children(&entity).await;
				let p_entity = world.entity(children[0]);
				// verify it's a <p> element
				let element: Element = p_entity
					.with_then(|entity| {
						entity.get::<Element>().cloned().unwrap()
					})
					.await;
				element.name().to_string()
			})
			.await
			.xpect_eq("p".to_string());
	}

	#[beet_core::test]
	async fn parse_heading() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				MarkdownParser::new()
					.parse(entity, b"# Title".to_vec(), None)
					.await
					.unwrap();
				let children = get_children(&entity).await;
				let h1 = world.entity(children[0]);
				let element: Element = h1
					.with_then(|entity| {
						entity.get::<Element>().cloned().unwrap()
					})
					.await;
				element.name().to_string()
			})
			.await
			.xpect_eq("h1".to_string());
	}

	#[beet_core::test]
	async fn parse_multiple_blocks() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				MarkdownParser::new()
					.parse(entity, b"# Title\n\nParagraph text".to_vec(), None)
					.await
					.unwrap();
				let children = get_children(&entity).await;
				children.len()
			})
			.await
			.xpect_eq(2);
	}

	#[beet_core::test]
	async fn parse_emphasis() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				MarkdownParser::new()
					.parse(entity, b"*hello*".to_vec(), None)
					.await
					.unwrap();
				// root -> p -> em -> "hello"
				let children = get_children(&entity).await;
				let p_entity = world.entity(children[0]);
				let p_children = get_children(&p_entity).await;
				let em = world.entity(p_children[0]);
				let element: Element = em
					.with_then(|entity| {
						entity.get::<Element>().cloned().unwrap()
					})
					.await;
				element.name().to_string()
			})
			.await
			.xpect_eq("em".to_string());
	}

	#[beet_core::test]
	async fn parse_link() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				MarkdownParser::new()
					.parse(
						entity,
						b"[click](https://example.com)".to_vec(),
						None,
					)
					.await
					.unwrap();
				// root -> p -> a
				let children = get_children(&entity).await;
				let p_entity = world.entity(children[0]);
				let p_children = get_children(&p_entity).await;
				let link = world.entity(p_children[0]);
				let element: Element = link
					.with_then(|entity| {
						entity.get::<Element>().cloned().unwrap()
					})
					.await;
				element.name().to_string()
			})
			.await
			.xpect_eq("a".to_string());
	}

	#[beet_core::test]
	async fn parse_code_block() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				MarkdownParser::new()
					.parse(entity, b"```rust\nfn main() {}\n```".to_vec(), None)
					.await
					.unwrap();
				let children = get_children(&entity).await;
				let pre = world.entity(children[0]);
				let element: Element = pre
					.with_then(|entity| {
						entity.get::<Element>().cloned().unwrap()
					})
					.await;
				element.name().to_string()
			})
			.await
			.xpect_eq("pre".to_string());
	}

	#[beet_core::test]
	async fn parse_unordered_list() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				MarkdownParser::new()
					.parse(entity, b"- item 1\n- item 2".to_vec(), None)
					.await
					.unwrap();
				let children = get_children(&entity).await;
				let ul = world.entity(children[0]);
				let element: Element = ul
					.with_then(|entity| {
						entity.get::<Element>().cloned().unwrap()
					})
					.await;
				let ul_children = get_children(&ul).await;
				(element.name().to_string(), ul_children.len())
			})
			.await
			.xpect_eq(("ul".to_string(), 2));
	}

	#[beet_core::test]
	async fn parse_with_path_inserts_file_span() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				MarkdownParser::new()
					.parse(
						entity,
						b"# Hello".to_vec(),
						Some(WsPathBuf::new("test.md")),
					)
					.await
					.unwrap();
				entity.get_cloned::<FileSpan>().await.unwrap()
			})
			.await
			.path()
			.xpect_eq(WsPathBuf::new("test.md"));
	}

	#[beet_core::test]
	async fn parse_yaml_frontmatter() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				MarkdownParser::new()
					.parse(
						entity,
						b"---\ntitle: Hello\nauthor: World\n---\n\n# Hello"
							.to_vec(),
						None,
					)
					.await
					.unwrap();
				let has_frontmatter: bool = entity
					.with_then(|entity| entity.get::<Frontmatter>().is_some())
					.await;
				has_frontmatter
			})
			.await
			.xpect_true();
	}

	#[beet_core::test]
	async fn parse_thematic_break() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				MarkdownParser::new()
					.parse(entity, b"---".to_vec(), None)
					.await
					.unwrap();
				let children = get_children(&entity).await;
				let hr = world.entity(children[0]);
				let element: Element = hr
					.with_then(|entity| {
						entity.get::<Element>().cloned().unwrap()
					})
					.await;
				element.name().to_string()
			})
			.await
			.xpect_eq("hr".to_string());
	}

	#[beet_core::test]
	async fn parse_image() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				MarkdownParser::new()
					.parse(entity, b"![alt text](image.png)".to_vec(), None)
					.await
					.unwrap();
				// root -> p -> img
				let children = get_children(&entity).await;
				let p_entity = world.entity(children[0]);
				let p_children = get_children(&p_entity).await;
				let img = world.entity(p_children[0]);
				let element: Element = img
					.with_then(|entity| {
						entity.get::<Element>().cloned().unwrap()
					})
					.await;
				element.name().to_string()
			})
			.await
			.xpect_eq("img".to_string());
	}

	#[beet_core::test]
	async fn reparse_unchanged_no_change() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				let mut parser = MarkdownParser::new();
				let md = b"# Title\n\nParagraph".to_vec();
				parser.parse(entity, md.clone(), None).await.unwrap();
				parser.parse(entity, md, None).await.unwrap();
				let children = get_children(&entity).await;
				children.len()
			})
			.await
			.xpect_eq(2);
	}

	#[beet_core::test]
	async fn parse_table() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				MarkdownParser::new()
					.parse(
						entity,
						b"| A | B |\n|---|---|\n| 1 | 2 |".to_vec(),
						None,
					)
					.await
					.unwrap();
				let children = get_children(&entity).await;
				let table = world.entity(children[0]);
				let element: Element = table
					.with_then(|entity| {
						entity.get::<Element>().cloned().unwrap()
					})
					.await;
				element.name().to_string()
			})
			.await
			.xpect_eq("table".to_string());
	}

	/// Helper to parse markdown then render it back via [`HtmlRenderer`].
	async fn roundtrip(md: &[u8]) -> String {
		let mut world_handle = AsyncPlugin::world();
		let md_owned = md.to_vec();
		world_handle
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				MarkdownParser::new()
					.parse(entity, md_owned, None)
					.await
					.unwrap();

				let id = entity.id();
				world
					.with_then(move |world| {
						let mut renderer = Some(HtmlRenderer::new());
						world
							.run_system_once(move |walker: NodeWalker| {
								let mut render = renderer.take().unwrap();
								walker.walk(&mut render, id);
								render.into_string()
							})
							.unwrap()
					})
					.await
			})
			.await
	}

	#[beet_core::test]
	async fn roundtrip_paragraph() {
		roundtrip(b"Hello world")
			.await
			.xpect_eq("<p>Hello world</p>".to_string());
	}

	#[beet_core::test]
	async fn roundtrip_heading() {
		roundtrip(b"# Title")
			.await
			.xpect_eq("<h1>Title</h1>".to_string());
	}

	#[beet_core::test]
	async fn roundtrip_emphasis() {
		roundtrip(b"*hello*")
			.await
			.xpect_eq("<p><em>hello</em></p>".to_string());
	}

	#[beet_core::test]
	async fn roundtrip_strong() {
		roundtrip(b"**hello**")
			.await
			.xpect_eq("<p><strong>hello</strong></p>".to_string());
	}

	#[beet_core::test]
	async fn roundtrip_link() {
		let html = roundtrip(b"[click](https://example.com)").await;
		html.xpect_contains("<a")
			.xpect_contains("href=\"https://example.com\"")
			.xpect_contains("click")
			.xpect_contains("</a>");
	}

	#[beet_core::test]
	async fn roundtrip_unordered_list() {
		let html = roundtrip(b"- a\n- b").await;
		html.xpect_contains("<ul>")
			.xpect_contains("<li>")
			.xpect_contains("</li>")
			.xpect_contains("</ul>");
	}

	#[beet_core::test]
	async fn roundtrip_code_block() {
		let html = roundtrip(b"```rust\nfn main() {}\n```").await;
		html.xpect_contains("<pre>")
			.xpect_contains("<code")
			.xpect_contains("</code>")
			.xpect_contains("</pre>");
	}

	#[beet_core::test]
	async fn roundtrip_blockquote() {
		let html = roundtrip(b"> quoted text").await;
		html.xpect_contains("<blockquote>")
			.xpect_contains("quoted text")
			.xpect_contains("</blockquote>");
	}

	#[beet_core::test]
	async fn roundtrip_thematic_break() {
		roundtrip(b"---").await.xpect_contains("<hr />");
	}

	#[beet_core::test]
	async fn roundtrip_image() {
		let html = roundtrip(b"![alt](image.png)").await;
		html.xpect_contains("<img")
			.xpect_contains("src=\"image.png\"");
	}

	#[beet_core::test]
	async fn roundtrip_inline_code() {
		roundtrip(b"use `foo()` here")
			.await
			.xpect_contains("<code>foo()</code>");
	}

	#[beet_core::test]
	async fn parse_embedded_html_block() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				// Markdown with an embedded HTML block
				let md =
					b"# Title\n\n<div class=\"custom\">inner</div>\n\nAfter"
						.to_vec();
				MarkdownParser::new().parse(entity, md, None).await.unwrap();
				let children = get_children(&entity).await;
				// Should have: h1, raw html text, paragraph
				(children.len() >= 2).xpect_true();
				// First child is the heading
				let h1 = world.entity(children[0]);
				let element: Element = h1
					.with_then(|entity| {
						entity.get::<Element>().cloned().unwrap()
					})
					.await;
				element.name().to_string()
			})
			.await
			.xpect_eq("h1".to_string());
	}

	#[beet_core::test]
	async fn parse_inline_html_in_markdown() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				// Inline HTML mixed with markdown
				MarkdownParser::new()
					.parse(
						entity,
						b"Hello <strong>world</strong> end".to_vec(),
						None,
					)
					.await
					.unwrap();
				let children = get_children(&entity).await;
				// Should produce a paragraph with mixed children
				children.len().xpect_eq(1);
				let p_entity = world.entity(children[0]);
				let p_children = get_children(&p_entity).await;
				// Multiple inline children: text, raw html, text, raw html, text
				(p_children.len() >= 3).xpect_true();
				true
			})
			.await
			.xpect_true();
	}

	#[beet_core::test]
	async fn span_tracking_heading() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				MarkdownParser::new()
					.parse(
						entity,
						b"# Title".to_vec(),
						Some(WsPathBuf::new("test.md")),
					)
					.await
					.unwrap();
				let children = get_children(&entity).await;
				let h1 = world.entity(children[0]);
				let span: Option<FileSpan> = h1
					.with_then(|entity| entity.get::<FileSpan>().cloned())
					.await;
				span.is_some()
			})
			.await
			.xpect_true();
	}

	#[beet_core::test]
	async fn span_tracking_text_node() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				MarkdownParser::new()
					.parse(
						entity,
						b"# Title".to_vec(),
						Some(WsPathBuf::new("test.md")),
					)
					.await
					.unwrap();
				let children = get_children(&entity).await;
				let h1 = world.entity(children[0]);
				let h1_children = get_children(&h1).await;
				let text = world.entity(h1_children[0]);
				let span: Option<FileSpan> = text
					.with_then(|entity| entity.get::<FileSpan>().cloned())
					.await;
				span.is_some()
			})
			.await
			.xpect_true();
	}

	#[beet_core::test]
	async fn span_tracking_multiline() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				// line 1: # Title\n
				// line 2: \n
				// line 3: Paragraph
				MarkdownParser::new()
					.parse(
						entity,
						b"# Title\n\nParagraph".to_vec(),
						Some(WsPathBuf::new("test.md")),
					)
					.await
					.unwrap();
				let children = get_children(&entity).await;
				// Check that the paragraph element has a span
				// starting after the heading
				let para = world.entity(children[1]);
				let span: FileSpan = para
					.with_then(|entity| {
						entity.get::<FileSpan>().cloned().unwrap()
					})
					.await;
				// paragraph starts on line 3
				(span.start().line >= 3).xpect_true();
				true
			})
			.await
			.xpect_true();
	}

	#[beet_core::test]
	async fn span_tracking_root_full_span() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				MarkdownParser::new()
					.parse(
						entity,
						b"# Title\n\nParagraph".to_vec(),
						Some(WsPathBuf::new("test.md")),
					)
					.await
					.unwrap();
				let span = entity.get_cloned::<FileSpan>().await.unwrap();
				// root span should cover entire input
				span.start().xpect_eq(LineCol::new(1, 0));
				span.path().clone()
			})
			.await
			.xpect_eq(WsPathBuf::new("test.md"));
	}
}
