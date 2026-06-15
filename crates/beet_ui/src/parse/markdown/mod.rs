//! Markdown parser producing an ECS entity tree of [`Element`]/[`Value`] nodes.
//!
//! Uses `pulldown-cmark` for markdown prose (headings, lists, emphasis, code),
//! building an [`HtmlNode`](diff::HtmlNode) intermediate representation that the
//! [`diff`] engine reconciles against existing entities on reparse. Markdown
//! owns the prose structure and the interleaving stack ([`tree_builder`]); the
//! markup itself (tags, attributes, `bx:` directives, spreads, components) is
//! parsed by the core BSX fragment primitive ([`parse_fragment`]), so BSX is the
//! single markup authority. An embedded uppercase component/template or `bx:`
//! directive resolves per-tag through the BSX resolver during the diff.
//!
//! Enable with the `markdown_parser` feature flag.

mod diff;
mod frontmatter;
mod tree_builder;
pub use diff::*;
pub use frontmatter::*;

use crate::prelude::*;
use beet_core::prelude::*;
use diff::diff_children;
use pulldown_cmark::Options;

/// Configuration for parsing embedded markup inside markdown, a thin wrapper over
/// the core [`BsxFragmentConfig`] threaded through to [`parse_fragment`].
pub type HtmlParseConfig = BsxFragmentConfig;

/// Helpers mirroring the markdown parser's expectations on [`HtmlParseConfig`].
pub trait HtmlParseConfigExt {
	/// A config with `{expr}` and `{{expr}}` expression parsing enabled.
	fn with_expressions() -> Self;
}

impl HtmlParseConfigExt for HtmlParseConfig {
	fn with_expressions() -> Self {
		Self {
			expressions: true,
			raw_text_expressions: true,
			..Default::default()
		}
	}
}

/// A configurable markdown parser that implements [`NodeParser`].
///
/// Parses markdown into a tree of ECS entities via an [`HtmlNode`]
/// representation and the markdown-owned diff engine. Embedded markup is parsed
/// by the core BSX fragment primitive ([`parse_fragment`]); an embedded
/// component/template tag or `bx:` directive resolves per-tag through the core
/// BSX resolver during the diff.
///
/// ## Example
/// ```rust
/// # use beet_ui::prelude::*;
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
	pub options: pulldown_cmark::Options,
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

		// diff tree against entity. an embedded uppercase component/template or
		// `bx:` directive resolves through the BSX resolver per-tag inside the
		// diff (no separate MDX resolution pass).
		diff_children(
			world,
			entity,
			&tree_result.nodes,
			&self.html_diff_config,
			span_lookup.as_ref(),
		)?;

		// run post-parse systems (syntax highlighting, style resolution, ..)
		// when registered, ie via `StylePlugin`.
		let _ = world.try_run_schedule(PostParseTree);

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
	fn parse(&mut self, cx: ParseContext) -> Result<(), ParseError> {
		let media_type = cx.bytes.media_type();
		if *media_type != MediaType::Markdown {
			return Err(ParseError::UnsupportedType {
				unsupported: media_type.clone(),
				supported: vec![MediaType::Markdown],
			});
		}
		let text = cx.bytes.as_utf8()?;
		let id = cx.entity.id();
		cx.entity
			.world_scope(|world| {
				self.parse_text(world, id, text, cx.path.as_ref())
			})?
			.xok()
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Parse markdown bytes into an entity.
	fn parse_md(entity: &mut EntityWorldMut, md: &str) {
		let bytes = MediaBytes::new_markdown(md);
		MarkdownParser::new()
			.parse(ParseContext::new(entity, &bytes))
			.unwrap();
	}

	/// Parse markdown with a path for span tracking.
	fn parse_md_with_path(entity: &mut EntityWorldMut, md: &str, path: &str) {
		let bytes = MediaBytes::new_markdown(md);
		MarkdownParser::new()
			.parse(
				ParseContext::new(entity, &bytes)
					.with_path(WsPathBuf::new(path)),
			)
			.unwrap();
	}

	#[beet_core::test]
	fn parse_simple_paragraph() {
		World::new()
			.spawn_empty()
			.xtap(|entity| parse_md(entity, "Hello world"))
			.children()
			.len()
			.xpect_eq(1);
	}

	#[beet_core::test]
	fn parse_paragraph_contains_text() {
		World::new()
			.spawn_empty()
			.xtap(|entity| parse_md(entity, "Hello world"))
			.child(0)
			.unwrap()
			.get::<Element>()
			.unwrap()
			.tag()
			.to_string()
			.xpect_eq("p".to_string());
	}

	#[beet_core::test]
	fn parse_heading() {
		World::new()
			.spawn_empty()
			.xtap(|entity| parse_md(entity, "# Title"))
			.child(0)
			.unwrap()
			.get::<Element>()
			.unwrap()
			.tag()
			.to_string()
			.xpect_eq("h1".to_string());
	}

	#[beet_core::test]
	fn parse_multiple_blocks() {
		World::new()
			.spawn_empty()
			.xtap(|entity| parse_md(entity, "# Title\n\nParagraph text"))
			.children()
			.len()
			.xpect_eq(2);
	}

	#[beet_core::test]
	fn parse_emphasis() {
		// root -> p -> em
		World::new()
			.spawn_empty()
			.xtap(|entity| parse_md(entity, "*hello*"))
			.child(0)
			.unwrap()
			.child(0)
			.unwrap()
			.get::<Element>()
			.unwrap()
			.tag()
			.to_string()
			.xpect_eq("em".to_string());
	}

	#[beet_core::test]
	fn parse_link() {
		// root -> p -> a
		World::new()
			.spawn_empty()
			.xtap(|entity| parse_md(entity, "[click](https://example.com)"))
			.child(0)
			.unwrap()
			.child(0)
			.unwrap()
			.get::<Element>()
			.unwrap()
			.tag()
			.to_string()
			.xpect_eq("a".to_string());
	}

	#[beet_core::test]
	fn parse_code_block() {
		World::new()
			.spawn_empty()
			.xtap(|entity| parse_md(entity, "```rust\nfn main() {}\n```"))
			.child(0)
			.unwrap()
			.get::<Element>()
			.unwrap()
			.tag()
			.to_string()
			.xpect_eq("pre".to_string());
	}

	#[beet_core::test]
	fn parse_unordered_list() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		parse_md(&mut world.entity_mut(entity), "- item 1\n- item 2");
		let ul = world.entity_mut(entity).child(0).unwrap().id();
		world
			.entity(ul)
			.get::<Element>()
			.unwrap()
			.tag()
			.xpect_eq("ul");
		world.entity_mut(ul).children().len().xpect_eq(2);
	}

	#[beet_core::test]
	fn parse_with_path_inserts_file_span() {
		World::new()
			.spawn_empty()
			.xtap(|entity| parse_md_with_path(entity, "# Hello", "test.md"))
			.get::<FileSpan>()
			.cloned()
			.unwrap()
			.path()
			.xpect_eq(SmolPath::new("test.md"));
	}

	#[beet_core::test]
	fn parse_yaml_frontmatter() {
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				parse_md(
					entity,
					"---\ntitle: Hello\nauthor: World\n---\n\n# Hello",
				)
			})
			.get::<Frontmatter>()
			.is_some()
			.xpect_true();
	}

	#[beet_core::test]
	fn parse_thematic_break() {
		World::new()
			.spawn_empty()
			.xtap(|entity| parse_md(entity, "---"))
			.child(0)
			.unwrap()
			.get::<Element>()
			.unwrap()
			.tag()
			.to_string()
			.xpect_eq("hr".to_string());
	}

	#[beet_core::test]
	fn parse_image() {
		// root -> p -> img
		World::new()
			.spawn_empty()
			.xtap(|entity| parse_md(entity, "![alt text](image.png)"))
			.child(0)
			.unwrap()
			.child(0)
			.unwrap()
			.get::<Element>()
			.unwrap()
			.tag()
			.to_string()
			.xpect_eq("img".to_string());
	}

	#[beet_core::test]
	fn reparse_unchanged_no_change() {
		let bytes = MediaBytes::new_markdown("# Title\n\nParagraph");
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				let mut parser = MarkdownParser::new();
				parser.parse(ParseContext::new(entity, &bytes)).unwrap();
				parser.parse(ParseContext::new(entity, &bytes)).unwrap();
			})
			.children()
			.len()
			.xpect_eq(2);
	}

	#[beet_core::test]
	fn parse_table() {
		World::new()
			.spawn_empty()
			.xtap(|entity| parse_md(entity, "| A | B |\n|---|---|\n| 1 | 2 |"))
			.child(0)
			.unwrap()
			.get::<Element>()
			.unwrap()
			.tag()
			.to_string()
			.xpect_eq("table".to_string());
	}

	/// Parse markdown then render it back via [`HtmlRenderer`].
	fn roundtrip(md: &str) -> String {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		parse_md(&mut world.entity_mut(entity), md);
		HtmlRenderer::new()
			.render(&mut RenderContext::new(entity, &mut world))
			.unwrap()
			.to_string()
	}

	#[beet_core::test]
	fn roundtrip_paragraph() {
		roundtrip("Hello world").xpect_eq("<p>Hello world</p>".to_string());
	}

	#[beet_core::test]
	fn roundtrip_heading() {
		roundtrip("# Title").xpect_eq("<h1>Title</h1>".to_string());
	}

	#[beet_core::test]
	fn roundtrip_emphasis() {
		roundtrip("*hello*").xpect_eq("<p><em>hello</em></p>".to_string());
	}

	#[beet_core::test]
	fn roundtrip_strong() {
		roundtrip("**hello**")
			.xpect_eq("<p><strong>hello</strong></p>".to_string());
	}

	#[beet_core::test]
	fn roundtrip_link() {
		let html = roundtrip("[click](https://example.com)");
		html.xpect_contains("<a")
			.xpect_contains("href=\"https://example.com\"")
			.xpect_contains("click")
			.xpect_contains("</a>");
	}

	#[beet_core::test]
	fn roundtrip_unordered_list() {
		let html = roundtrip("- a\n- b");
		html.xpect_contains("<ul>")
			.xpect_contains("<li>")
			.xpect_contains("</li>")
			.xpect_contains("</ul>");
	}

	#[beet_core::test]
	fn roundtrip_code_block() {
		let html = roundtrip("```rust\nfn main() {}\n```");
		html.xpect_contains("<pre>")
			.xpect_contains("<code")
			.xpect_contains("</code>")
			.xpect_contains("</pre>");
	}

	#[beet_core::test]
	fn roundtrip_blockquote() {
		let html = roundtrip("> quoted text");
		html.xpect_contains("<blockquote>")
			.xpect_contains("quoted text")
			.xpect_contains("</blockquote>");
	}

	#[beet_core::test]
	fn roundtrip_thematic_break() { roundtrip("---").xpect_contains("<hr />"); }

	#[beet_core::test]
	fn roundtrip_image() {
		let html = roundtrip("![alt](image.png)");
		html.xpect_contains("<img")
			.xpect_contains("src=\"image.png\"");
	}

	#[beet_core::test]
	fn roundtrip_inline_code() {
		roundtrip("use `foo()` here").xpect_contains("<code>foo()</code>");
	}

	#[beet_core::test]
	fn parse_embedded_html_block() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		parse_md(
			&mut world.entity_mut(entity),
			"# Title\n\n<div class=\"custom\">inner</div>\n\nAfter",
		);
		// Should have: h1, raw html text, paragraph
		(world.entity_mut(entity).children().len() >= 2).xpect_true();
		world
			.entity_mut(entity)
			.child(0)
			.unwrap()
			.get::<Element>()
			.unwrap()
			.tag()
			.to_string()
			.xpect_eq("h1".to_string());
	}

	#[beet_core::test]
	fn parse_inline_html_in_markdown() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		parse_md(
			&mut world.entity_mut(entity),
			"Hello <strong>world</strong> end",
		);
		// Should produce a paragraph with mixed children
		world.entity_mut(entity).children().len().xpect_eq(1);
		// Multiple inline children
		(world.entity_mut(entity).child(0).unwrap().children().len() >= 3)
			.xpect_true();
	}

	#[beet_core::test]
	fn span_tracking_heading() {
		World::new()
			.spawn_empty()
			.xtap(|entity| parse_md_with_path(entity, "# Title", "test.md"))
			.child(0)
			.unwrap()
			.get::<FileSpan>()
			.is_some()
			.xpect_true();
	}

	#[beet_core::test]
	fn span_tracking_text_node() {
		World::new()
			.spawn_empty()
			.xtap(|entity| parse_md_with_path(entity, "# Title", "test.md"))
			.child(0)
			.unwrap()
			.child(0)
			.unwrap()
			.get::<FileSpan>()
			.is_some()
			.xpect_true();
	}

	#[beet_core::test]
	fn span_tracking_multiline() {
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				// line 1: # Title\n
				// line 2: \n
				// line 3: Paragraph
				parse_md_with_path(entity, "# Title\n\nParagraph", "test.md")
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

	#[beet_core::test]
	fn span_tracking_root_full_span() {
		let span = World::new()
			.spawn_empty()
			.xtap(|entity| {
				parse_md_with_path(entity, "# Title\n\nParagraph", "test.md")
			})
			.get::<FileSpan>()
			.cloned()
			.unwrap();
		// root span should cover entire input
		span.start().xpect_eq(LineCol::new(1, 0));
		span.path().xpect_eq(SmolPath::new("test.md"));
	}

	/// Root entity must not receive any content components — only metadata
	/// like [`Frontmatter`] and [`FileSpan`]. All parsed content lives in
	/// children.
	#[beet_core::test]
	fn root_has_no_content_components_heading() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		parse_md(&mut world.entity_mut(entity), "# Title");
		let root = world.entity(entity);
		root.get::<Element>().is_none().xpect_true();
		root.get::<Value>().is_none().xpect_true();
		root.get::<Comment>().is_none().xpect_true();
		root.get::<Expression>().is_none().xpect_true();
	}

	#[beet_core::test]
	fn root_has_no_content_components_paragraph() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		parse_md(&mut world.entity_mut(entity), "Hello world");
		let root = world.entity(entity);
		root.get::<Element>().is_none().xpect_true();
		root.get::<Value>().is_none().xpect_true();
		root.get::<Comment>().is_none().xpect_true();
		root.get::<Expression>().is_none().xpect_true();
	}

	#[cfg(all(feature = "syntax_highlighting", not(target_arch = "wasm32")))]
	#[beet_core::test]
	fn syntax_highlighting_replaces_text_with_spans() {
		let mut app = App::new();
		app.add_plugins(StylePlugin);
		let entity = app.world_mut().spawn_empty().id();
		let bytes = MediaBytes::new_markdown("```rust\nfn main() {}\n```");
		MarkdownParser::new()
			.parse(ParseContext::new(
				&mut app.world_mut().entity_mut(entity),
				&bytes,
			))
			.unwrap();

		// root -> pre -> code -> spans (no plain text child)
		let world = app.world_mut();
		let pre = world.entity_mut(entity).child(0).unwrap().id();
		let code = world.entity_mut(pre).child(0).unwrap().id();
		let code_children: Vec<_> = world
			.entity(code)
			.get::<Children>()
			.map(|c| c.iter().collect())
			.unwrap_or_default();
		(code_children.len() >= 2).xpect_true();
		// every direct child of the code element is a span element
		for child in code_children {
			world
				.entity(child)
				.get::<Element>()
				.unwrap()
				.tag()
				.xpect_eq("span");
		}
	}

	#[beet_core::test]
	fn root_has_no_content_components_multiple_blocks() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		parse_md(
			&mut world.entity_mut(entity),
			"# Title\n\nParagraph\n\n- list item",
		);
		let root = world.entity(entity);
		root.get::<Element>().is_none().xpect_true();
		root.get::<Value>().is_none().xpect_true();
		// all three blocks are children, not on the root
		world.entity_mut(entity).children().len().xpect_eq(3);
	}
}
