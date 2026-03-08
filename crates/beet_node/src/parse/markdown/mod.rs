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
mod tree_builder;
pub(crate) use tree_builder::*;

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
#[derive(Debug, Default, Clone)]
pub struct MarkdownParser {
	pub html_parse_config: HtmlParseConfig,
	pub html_diff_config: HtmlDiffConfig,
	pub config: MarkdownParseConfig,
}

#[derive(Debug, Clone)]
pub struct MarkdownParseConfig {
	/// pulldown-cmark options controlling which extensions are enabled.
	pub options: Options,
	/// Whether to parse frontmatter metadata blocks.
	pub parse_frontmatter: bool,
}

impl Default for MarkdownParseConfig {
	fn default() -> Self {
		Self {
			options: Self::default_cmark_options(),
			parse_frontmatter: true,
		}
	}
}

impl MarkdownParseConfig {
	/// Returns the default pulldown-cmark options with maximal extensions.
	pub fn default_cmark_options() -> Options {
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
}


impl MarkdownParser {
	/// Create a new parser with default settings and maximal extensions.
	pub fn new() -> Self { Self::default() }

	/// Create a parser with expression support enabled in markdown text
	/// and embedded HTML, ie rusty mdx.
	pub fn with_expressions() -> Self {
		Self {
			html_parse_config: HtmlParseConfig::with_expressions(),
			..Default::default()
		}
	}


	/// Shared parsing logic: tokenize markdown, build tree, diff against entity.
	fn parse_text(
		&self,
		world: &mut World,
		entity: Entity,
		text: &str,
		path: Option<&WsPathBuf>,
	) -> Result {
		let span_lookup = path.map(|path| SpanLookup::new(text, path.clone()));

		let tree_result = tree_builder::build_markdown_tree(
			text,
			self.config.options,
			&self.html_parse_config,
			&self.html_diff_config,
			span_lookup.as_ref(),
		)?;

		// diff tree against entity
		diff_children(
			world,
			entity,
			&tree_result.nodes,
			&self.html_diff_config,
			span_lookup.as_ref(),
		)?;

		// insert frontmatter on root if present
		if self.config.parse_frontmatter {
			if let Some(fm) = tree_result.frontmatter {
				world.entity_mut(entity).insert(fm);
			} else {
				// remove stale frontmatter if content no longer has it
				world.entity_mut(entity).remove::<Frontmatter>();
			}
		}

		// insert file span on the root entity if path provided
		if let Some(ref lookup) = span_lookup {
			let span = lookup.full_span();
			world.entity_mut(entity).set_if_ne_or_insert(span);
		}

		Ok(())
	}
}

impl NodeParser for MarkdownParser {
	fn parse(
		&mut self,
		entity: &mut EntityWorldMut,
		bytes: &[u8],
		path: Option<WsPathBuf>,
	) -> Result {
		let text = std::str::from_utf8(&bytes)?;
		let id = entity.id();
		entity.world_scope(|world| {
			self.parse_text(world, id, text, path.as_ref())
		})
	}
}

// Re-export diff internals used by tree_builder within crate
use crate::parse::html::diff::diff_children;



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn parse_simple_paragraph() {
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				MarkdownParser::new()
					.parse(entity, b"Hello world", None)
					.unwrap();
			})
			.children()
			.len()
			.xpect_eq(1);
	}

	#[test]
	fn parse_paragraph_contains_text() {
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				MarkdownParser::new()
					.parse(entity, b"Hello world", None)
					.unwrap();
			})
			.child(0)
			.unwrap()
			.get::<Element>()
			.unwrap()
			.name()
			.to_string()
			.xpect_eq("p".to_string());
	}

	#[test]
	fn parse_heading() {
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				MarkdownParser::new()
					.parse(entity, b"# Title", None)
					.unwrap();
			})
			.child(0)
			.unwrap()
			.get::<Element>()
			.unwrap()
			.name()
			.to_string()
			.xpect_eq("h1".to_string());
	}

	#[test]
	fn parse_multiple_blocks() {
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				MarkdownParser::new()
					.parse(entity, b"# Title\n\nParagraph text", None)
					.unwrap();
			})
			.children()
			.len()
			.xpect_eq(2);
	}

	#[test]
	fn parse_emphasis() {
		// root -> p -> em
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				MarkdownParser::new()
					.parse(entity, b"*hello*", None)
					.unwrap();
			})
			.child(0)
			.unwrap()
			.child(0)
			.unwrap()
			.get::<Element>()
			.unwrap()
			.name()
			.to_string()
			.xpect_eq("em".to_string());
	}

	#[test]
	fn parse_link() {
		// root -> p -> a
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				MarkdownParser::new()
					.parse(entity, b"[click](https://example.com)", None)
					.unwrap();
			})
			.child(0)
			.unwrap()
			.child(0)
			.unwrap()
			.get::<Element>()
			.unwrap()
			.name()
			.to_string()
			.xpect_eq("a".to_string());
	}

	#[test]
	fn parse_code_block() {
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				MarkdownParser::new()
					.parse(entity, b"```rust\nfn main() {}\n```", None)
					.unwrap();
			})
			.child(0)
			.unwrap()
			.get::<Element>()
			.unwrap()
			.name()
			.to_string()
			.xpect_eq("pre".to_string());
	}

	#[test]
	fn parse_unordered_list() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		MarkdownParser::new()
			.parse(&mut world.entity_mut(entity), b"- item 1\n- item 2", None)
			.unwrap();
		let ul = world.entity_mut(entity).child(0).unwrap().id();
		world
			.entity(ul)
			.get::<Element>()
			.unwrap()
			.name()
			.xpect_eq("ul");
		world.entity_mut(ul).children().len().xpect_eq(2);
	}

	#[test]
	fn parse_with_path_inserts_file_span() {
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				MarkdownParser::new()
					.parse(entity, b"# Hello", Some(WsPathBuf::new("test.md")))
					.unwrap();
			})
			.get::<FileSpan>()
			.cloned()
			.unwrap()
			.path()
			.xpect_eq(WsPathBuf::new("test.md"));
	}

	#[test]
	fn parse_yaml_frontmatter() {
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				MarkdownParser::new()
					.parse(
						entity,
						b"---\ntitle: Hello\nauthor: World\n---\n\n# Hello",
						None,
					)
					.unwrap();
			})
			.get::<Frontmatter>()
			.is_some()
			.xpect_true();
	}

	#[test]
	fn parse_thematic_break() {
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				MarkdownParser::new().parse(entity, b"---", None).unwrap();
			})
			.child(0)
			.unwrap()
			.get::<Element>()
			.unwrap()
			.name()
			.to_string()
			.xpect_eq("hr".to_string());
	}

	#[test]
	fn parse_image() {
		// root -> p -> img
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				MarkdownParser::new()
					.parse(entity, b"![alt text](image.png)", None)
					.unwrap();
			})
			.child(0)
			.unwrap()
			.child(0)
			.unwrap()
			.get::<Element>()
			.unwrap()
			.name()
			.to_string()
			.xpect_eq("img".to_string());
	}

	#[test]
	fn reparse_unchanged_no_change() {
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				let mut parser = MarkdownParser::new();
				parser.parse(entity, b"# Title\n\nParagraph", None).unwrap();
				parser.parse(entity, b"# Title\n\nParagraph", None).unwrap();
			})
			.children()
			.len()
			.xpect_eq(2);
	}

	#[test]
	fn parse_table() {
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				MarkdownParser::new()
					.parse(entity, b"| A | B |\n|---|---|\n| 1 | 2 |", None)
					.unwrap();
			})
			.child(0)
			.unwrap()
			.get::<Element>()
			.unwrap()
			.name()
			.to_string()
			.xpect_eq("table".to_string());
	}

	/// Parse markdown then render it back via [`HtmlRenderer`].
	fn roundtrip(md: &[u8]) -> String {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		MarkdownParser::new()
			.parse(&mut world.entity_mut(entity), md, None)
			.unwrap();
		world
			.run_system_once(move |walker: NodeWalker| {
				let mut renderer = HtmlRenderer::new();
				walker.walk(&mut renderer, entity);
				renderer.into_string()
			})
			.unwrap()
	}

	#[test]
	fn roundtrip_paragraph() {
		roundtrip(b"Hello world").xpect_eq("<p>Hello world</p>".to_string());
	}

	#[test]
	fn roundtrip_heading() {
		roundtrip(b"# Title").xpect_eq("<h1>Title</h1>".to_string());
	}

	#[test]
	fn roundtrip_emphasis() {
		roundtrip(b"*hello*").xpect_eq("<p><em>hello</em></p>".to_string());
	}

	#[test]
	fn roundtrip_strong() {
		roundtrip(b"**hello**")
			.xpect_eq("<p><strong>hello</strong></p>".to_string());
	}

	#[test]
	fn roundtrip_link() {
		let html = roundtrip(b"[click](https://example.com)");
		html.xpect_contains("<a")
			.xpect_contains("href=\"https://example.com\"")
			.xpect_contains("click")
			.xpect_contains("</a>");
	}

	#[test]
	fn roundtrip_unordered_list() {
		let html = roundtrip(b"- a\n- b");
		html.xpect_contains("<ul>")
			.xpect_contains("<li>")
			.xpect_contains("</li>")
			.xpect_contains("</ul>");
	}

	#[test]
	fn roundtrip_code_block() {
		let html = roundtrip(b"```rust\nfn main() {}\n```");
		html.xpect_contains("<pre>")
			.xpect_contains("<code")
			.xpect_contains("</code>")
			.xpect_contains("</pre>");
	}

	#[test]
	fn roundtrip_blockquote() {
		let html = roundtrip(b"> quoted text");
		html.xpect_contains("<blockquote>")
			.xpect_contains("quoted text")
			.xpect_contains("</blockquote>");
	}

	#[test]
	fn roundtrip_thematic_break() {
		roundtrip(b"---").xpect_contains("<hr />");
	}

	#[test]
	fn roundtrip_image() {
		let html = roundtrip(b"![alt](image.png)");
		html.xpect_contains("<img")
			.xpect_contains("src=\"image.png\"");
	}

	#[test]
	fn roundtrip_inline_code() {
		roundtrip(b"use `foo()` here").xpect_contains("<code>foo()</code>");
	}

	#[test]
	fn parse_embedded_html_block() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		MarkdownParser::new()
			.parse(
				&mut world.entity_mut(entity),
				b"# Title\n\n<div class=\"custom\">inner</div>\n\nAfter",
				None,
			)
			.unwrap();
		// Should have: h1, raw html text, paragraph
		(world.entity_mut(entity).children().len() >= 2).xpect_true();
		world
			.entity_mut(entity)
			.child(0)
			.unwrap()
			.get::<Element>()
			.unwrap()
			.name()
			.to_string()
			.xpect_eq("h1".to_string());
	}

	#[test]
	fn parse_inline_html_in_markdown() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		MarkdownParser::new()
			.parse(
				&mut world.entity_mut(entity),
				b"Hello <strong>world</strong> end",
				None,
			)
			.unwrap();
		// Should produce a paragraph with mixed children
		world.entity_mut(entity).children().len().xpect_eq(1);
		// Multiple inline children
		(world.entity_mut(entity).child(0).unwrap().children().len() >= 3)
			.xpect_true();
	}

	#[test]
	fn span_tracking_heading() {
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				MarkdownParser::new()
					.parse(entity, b"# Title", Some(WsPathBuf::new("test.md")))
					.unwrap();
			})
			.child(0)
			.unwrap()
			.get::<FileSpan>()
			.is_some()
			.xpect_true();
	}

	#[test]
	fn span_tracking_text_node() {
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				MarkdownParser::new()
					.parse(entity, b"# Title", Some(WsPathBuf::new("test.md")))
					.unwrap();
			})
			.child(0)
			.unwrap()
			.child(0)
			.unwrap()
			.get::<FileSpan>()
			.is_some()
			.xpect_true();
	}

	#[test]
	fn span_tracking_multiline() {
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				// line 1: # Title\n
				// line 2: \n
				// line 3: Paragraph
				MarkdownParser::new()
					.parse(
						entity,
						b"# Title\n\nParagraph",
						Some(WsPathBuf::new("test.md")),
					)
					.unwrap();
			})
			.child(1)
			.unwrap()
			.get::<FileSpan>()
			.cloned()
			.unwrap()
			.start()
			.line
			.xpect_eq(3);
	}

	#[test]
	fn span_tracking_root_full_span() {
		let span = World::new()
			.spawn_empty()
			.xtap(|entity| {
				MarkdownParser::new()
					.parse(
						entity,
						b"# Title\n\nParagraph",
						Some(WsPathBuf::new("test.md")),
					)
					.unwrap();
			})
			.get::<FileSpan>()
			.cloned()
			.unwrap();
		// root span should cover entire input
		span.start().xpect_eq(LineCol::new(1, 0));
		span.path().xpect_eq(WsPathBuf::new("test.md"));
	}
}
