//! Markdown and mdx as scene bodies.
//!
//! `pulldown-cmark` reads the prose (headings, lists, emphasis, code) and
//! [`tree_builder`] interleaves it with embedded markup, which the core BSX
//! fragment primitive ([`BsxFragmentToken::parse_fragment`]) reads, into the
//! [`BsxNode`] tree every markup format lowers from. So a component, a `bx:`
//! directive or a spread in markdown is the one it is in BSX, in one tree.
//!
//! A document's frontmatter is its root's components ([`Frontmatter`]).
//!
//! Enable with the `markdown_parser` feature flag.

mod tree_builder;

use crate::prelude::*;
use beet_core::prelude::*;
use pulldown_cmark::Options;
use tree_builder::MarkdownTree;

/// Reads markdown, or mdx ([`mdx`](Self::mdx)), as a scene body.
///
/// ## Example
/// ```rust
/// # use beet_ui::prelude::*;
/// # use beet_core::prelude::*;
/// let body = MarkdownParser::new()
/// 	.parse(&MediaBytes::new_markdown("# Hi"), &DocParsers::default())
/// 	.unwrap();
/// body.entities.keys().nth(1).unwrap().as_str().xpect_eq("0/h1.0");
/// ```
#[derive(Debug, Clone)]
pub struct MarkdownParser {
	media_type: MediaType,
	/// How embedded markup is read.
	fragment: BsxFragmentConfig,
	config: MarkdownParseConfig,
}

impl Default for MarkdownParser {
	fn default() -> Self {
		Self {
			media_type: MediaType::Markdown,
			fragment: default(),
			config: default(),
		}
	}
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
	/// Read markdown, with maximal extensions.
	pub fn new() -> Self { Self::default() }

	/// Read mdx: markdown whose text holds `{..}` expressions, as BSX text
	/// does, a heading's no attribute block.
	pub fn mdx() -> Self {
		Self {
			media_type: MediaType::Mdx,
			config: MarkdownParseConfig {
				options: MarkdownParseConfig::default_cmark_options()
					- Options::ENABLE_HEADING_ATTRIBUTES,
				..default()
			},
			fragment: BsxFragmentConfig {
				expressions: true,
				raw_text_expressions: true,
				..default()
			},
			..default()
		}
	}

	/// The markup nodes of `text` and its frontmatter, for a tool that stops
	/// before the body.
	pub fn nodes(
		&self,
		text: &str,
	) -> Result<(Vec<BsxNode>, Option<Frontmatter>)> {
		let tree = MarkdownTree::build(text, self)?;
		(tree.nodes, tree.frontmatter).xok()
	}
}

impl DocParser for MarkdownParser {
	fn media_types(&self) -> Vec<MediaType> { vec![self.media_type.clone()] }

	fn parse(
		&mut self,
		bytes: &MediaBytes,
		parsers: &DocParsers,
	) -> Result<SceneBody, ParseError> {
		if *bytes.media_type() != self.media_type {
			return Err(ParseError::UnsupportedType {
				unsupported: bytes.media_type().clone(),
				supported: self.media_types(),
			});
		}
		let (nodes, frontmatter) = self.nodes(bytes.as_utf8()?)?;
		let root = frontmatter
			.filter(|_| self.config.parse_frontmatter)
			.map(|frontmatter| frontmatter.components(parsers))
			.transpose()?
			.unwrap_or_default();
		BsxParser::lower_container(nodes, root, parsers)?.xok()
	}
}

impl Installer for MarkdownParser {
	fn install(self, app: &mut App) { DocParsers::register_on(app, self); }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// Parse markdown bytes into an entity.
	fn parse_md(entity: &mut EntityWorldMut, md: &str) {
		let bytes = MediaBytes::new_markdown(md);
		entity
			.world_scope(|world| world.insert_resource(ParsePlugin::parsers()));
		DocParsers::parse(ParseContext::new(entity, &bytes)).unwrap();
	}

	#[beet_core::test]
	fn parse_simple_paragraph() {
		TemplatePlugin::world()
			.spawn_empty()
			.xtap(|entity| parse_md(entity, "Hello world"))
			.children()
			.len()
			.xpect_eq(1);
	}

	#[beet_core::test]
	fn parse_paragraph_contains_text() {
		TemplatePlugin::world()
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
		TemplatePlugin::world()
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
		TemplatePlugin::world()
			.spawn_empty()
			.xtap(|entity| parse_md(entity, "# Title\n\nParagraph text"))
			.children()
			.len()
			.xpect_eq(2);
	}

	#[beet_core::test]
	fn parse_emphasis() {
		// root -> p -> em
		TemplatePlugin::world()
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
		TemplatePlugin::world()
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
		TemplatePlugin::world()
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
		let mut world = TemplatePlugin::world();
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
	fn parse_yaml_frontmatter() {
		ParsePlugin
			.into_world()
			.spawn_empty()
			.xtap(|entity| {
				parse_md(
					entity,
					"---\ntitle: Hello\ndescription: World\n---\n\n# Hello",
				)
			})
			.get::<DocMeta>()
			.unwrap()
			.title
			.as_deref()
			.xpect_eq(Some("Hello"));
	}

	#[beet_core::test]
	fn parse_thematic_break() {
		TemplatePlugin::world()
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
		TemplatePlugin::world()
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
	fn parse_table() {
		// root -> div.table-scroll -> table
		TemplatePlugin::world()
			.spawn_empty()
			.xtap(|entity| parse_md(entity, "| A | B |\n|---|---|\n| 1 | 2 |"))
			.child(0)
			.unwrap()
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
		let mut world = TemplatePlugin::world();
		let entity = world.spawn_empty().id();
		parse_md(&mut world.entity_mut(entity), md);
		HtmlRenderer::new()
			.render(&mut RenderContext::new(
				&mut world,
				entity,
				&RequestParts::default(),
			))
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
		let mut world = TemplatePlugin::world();
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

	/// An element one HTML block opens and a later one closes holds the
	/// markdown between them, ie a disclosure's body.
	#[beet_core::test]
	fn html_blocks_span_markdown() {
		roundtrip(
			"<details><summary>Open me</summary>\n\nHidden **body**.\n\n</details>\n\nAfter",
		)
		.xpect_contains("<details><summary>Open me</summary><p>Hidden <strong>body</strong>.</p></details><p>After</p>")
		.xnot()
		.xpect_contains("html_block");
	}

	#[beet_core::test]
	fn parse_inline_html_in_markdown() {
		let mut world = TemplatePlugin::world();
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

	/// The root holds the document's metadata alone, its content being its
	/// children.
	#[beet_core::test]
	fn root_has_no_content_components_heading() {
		let mut world = TemplatePlugin::world();
		let entity = world.spawn_empty().id();
		parse_md(&mut world.entity_mut(entity), "# Title");
		let root = world.entity(entity);
		root.get::<Element>().is_none().xpect_true();
		root.get::<Value>().is_none().xpect_true();
		root.get::<Comment>().is_none().xpect_true();
	}

	#[beet_core::test]
	fn root_has_no_content_components_paragraph() {
		let mut world = TemplatePlugin::world();
		let entity = world.spawn_empty().id();
		parse_md(&mut world.entity_mut(entity), "Hello world");
		let root = world.entity(entity);
		root.get::<Element>().is_none().xpect_true();
		root.get::<Value>().is_none().xpect_true();
		root.get::<Comment>().is_none().xpect_true();
	}

	#[cfg(all(feature = "syntax_highlighting", not(target_arch = "wasm32")))]
	#[beet_core::test]
	fn syntax_highlighting_replaces_text_with_spans() {
		let mut app = App::new();
		app.add_plugins(StylePlugin);
		let entity = app.world_mut().spawn_empty().id();
		let bytes = MediaBytes::new_markdown("```rust\nfn main() {}\n```");
		// highlighting is a post-parse pass, which the media parse runs
		DocParsers::parse(ParseContext::new(
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
		let mut world = TemplatePlugin::world();
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
