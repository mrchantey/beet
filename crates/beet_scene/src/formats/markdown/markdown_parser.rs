//! [`MarkdownParser`], markdown and mdx as scene bodies.
use super::tree_builder::MarkdownTree;
use crate::prelude::*;
use beet_core::prelude::*;
use pulldown_cmark::Options;

/// Reads markdown, or mdx ([`mdx`](Self::mdx)), as a scene body.
///
/// ## Example
/// ```rust
/// # use beet_core::prelude::*;
/// # use beet_scene::prelude::*;
/// let document = MarkdownParser::new()
/// 	.parse(&MediaBytes::new_markdown("# Hi"), &DocParsers::default())
/// 	.unwrap();
/// document.body.entities.keys().nth(1).unwrap().as_str().xpect_eq("0/h1.0");
/// ```
#[derive(Debug, Clone)]
pub struct MarkdownParser {
	media_type: MediaType,
	/// How embedded markup is read.
	pub(super) fragment: BsxFragmentConfig,
	pub(super) config: MarkdownParseConfig,
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
	/// before the body. A fence that renders is the document it holds, see the
	/// `markdown` module docs.
	pub fn nodes(
		&self,
		text: &str,
		parsers: &DocParsers,
	) -> Result<(Vec<BsxNode>, Option<Frontmatter>)> {
		let tree = MarkdownTree::build(text, self, parsers)?;
		(tree.nodes, tree.frontmatter).xok()
	}
}

impl DocParser for MarkdownParser {
	fn media_types(&self) -> Vec<MediaType> { vec![self.media_type.clone()] }

	fn parse(
		&mut self,
		bytes: &MediaBytes,
		parsers: &DocParsers,
	) -> Result<ParsedDocument, ParseError> {
		if *bytes.media_type() != self.media_type {
			return Err(ParseError::UnsupportedType {
				unsupported: bytes.media_type().clone(),
				supported: self.media_types(),
			});
		}
		let text = bytes.as_utf8()?;
		let (nodes, frontmatter) = self.nodes(text, parsers)?;
		let root = frontmatter
			.filter(|_| self.config.parse_frontmatter)
			.map(|frontmatter| frontmatter.components(parsers))
			.transpose()?
			.unwrap_or_default();
		let mut document = BsxParser::lower_container(nodes, root, parsers)?;
		// the root is the page whole
		document.spans.insert(EntityKey::ROOT, 0..text.len() as u32);
		document.xok()
	}
}

impl Installer for MarkdownParser {
	fn install(self, app: &mut App) { DocParsers::register_on(app, self); }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Parse markdown bytes into an entity.
	fn parse_md(entity: &mut EntityWorldMut, md: &str) {
		let bytes = MediaBytes::new_markdown(md);
		entity
			.world_scope(|world| world.insert_resource(ScenePlugin::parsers()));
		ParseContext::new(entity, &bytes).parse().unwrap();
	}

	#[beet_core::test]
	fn parse_simple_paragraph() {
		ScenePlugin::world()
			.spawn_empty()
			.xtap(|entity| parse_md(entity, "Hello world"))
			.children()
			.len()
			.xpect_eq(1);
	}

	#[beet_core::test]
	fn parse_paragraph_contains_text() {
		ScenePlugin::world()
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
		ScenePlugin::world()
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
		ScenePlugin::world()
			.spawn_empty()
			.xtap(|entity| parse_md(entity, "# Title\n\nParagraph text"))
			.children()
			.len()
			.xpect_eq(2);
	}

	#[beet_core::test]
	fn parse_emphasis() {
		// root -> p -> em
		ScenePlugin::world()
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
		ScenePlugin::world()
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
		ScenePlugin::world()
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
		let mut world = ScenePlugin::world();
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
	fn parse_thematic_break() {
		ScenePlugin::world()
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
		ScenePlugin::world()
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
		ScenePlugin::world()
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

	#[beet_core::test]
	fn parse_embedded_html_block() {
		let mut world = ScenePlugin::world();
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
		let mut world = ScenePlugin::world();
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
		let mut world = ScenePlugin::world();
		let entity = world.spawn_empty().id();
		parse_md(&mut world.entity_mut(entity), "# Title");
		let root = world.entity(entity);
		root.get::<Element>().is_none().xpect_true();
		root.get::<Value>().is_none().xpect_true();
		root.get::<Comment>().is_none().xpect_true();
	}

	#[beet_core::test]
	fn root_has_no_content_components_paragraph() {
		let mut world = ScenePlugin::world();
		let entity = world.spawn_empty().id();
		parse_md(&mut world.entity_mut(entity), "Hello world");
		let root = world.entity(entity);
		root.get::<Element>().is_none().xpect_true();
		root.get::<Value>().is_none().xpect_true();
		root.get::<Comment>().is_none().xpect_true();
	}

	#[beet_core::test]
	fn root_has_no_content_components_multiple_blocks() {
		let mut world = ScenePlugin::world();
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

	/// The body of `md` read by the base formats.
	fn body(md: &str) -> SceneBody {
		ScenePlugin::parsers()
			.body(&MediaBytes::new_markdown(md))
			.unwrap()
	}

	/// The keys of `body`, in document order.
	fn keys(body: &SceneBody) -> Vec<&str> {
		body.entities.keys().map(|key| key.as_str()).collect()
	}

	/// A fence that says `render` is that document, its root standing where
	/// the fence does and every reference within it following its target.
	#[beet_core::test]
	fn a_rendered_fence_is_its_document() {
		let body = body(
			"# Hi\n\n```application/vnd.beet.bsx render\n<main {Holds($there)}><i bx:ref=\"there\"/><b/></main>\n```\n\nafter",
		);
		keys(&body).xpect_eq(vec![
			"0",
			"0/h1.0",
			"0/h1.0/#text.0",
			"0/main.0",
			"0/main.0/there",
			"0/main.0/b.0",
			"0/p.0",
			"0/p.0/#text.0",
		]);
		let parent = |key: &str| {
			body.entities[&EntityKey::parse(key).unwrap()]
				.get("bevy_ecs::hierarchy::ChildOf")
				.unwrap()
				.clone()
		};
		parent("0/main.0").xpect_eq(Value::str("0"));
		parent("0/main.0/there").xpect_eq(Value::str("0/main.0"));
	}

	/// A component of a type the registry knows follows its target into the
	/// document holding the fence.
	#[beet_core::test]
	fn a_fences_references_follow() {
		#[derive(Debug, Clone, PartialEq, Component, Reflect, MapEntities)]
		#[reflect(Component, MapEntities)]
		struct Holds(#[entities] Entity);

		let mut world = ScenePlugin::world();
		world
			.resource::<AppTypeRegistry>()
			.write()
			.register::<Holds>();
		let root = world.spawn_empty().id();
		parse_md(
			&mut world.entity_mut(root),
			"```application/vnd.beet.bsx render\n<main {Holds($there)}><i bx:ref=\"there\"/></main>\n```",
		);
		let main = world.get::<Children>(root).unwrap()[0];
		let there = world.get::<Children>(main).unwrap()[0];
		world.get::<Holds>(main).unwrap().0.xpect_eq(there);
	}

	/// A fence that does not say `render` is a code sample, whether it names
	/// its format by language or by media type, and says what it likes after.
	#[beet_core::test]
	fn an_unrendered_fence_is_code() {
		for info in [
			"html",
			"text/html",
			"bsx",
			"json",
			"toml",
			"js",
			"md",
			"text",
			"",
			"html render=code",
			"json {1,3}",
		] {
			keys(&body(&format!("```{info}\n<b/>\n```")))
				.contains(&"0/pre.0/code.0")
				.xpect_true();
		}
	}

	/// A language names its format as its media type does.
	#[beet_core::test]
	fn a_language_renders_as_its_media_type() {
		for info in ["html render", "text/html render", "html render=scene"] {
			keys(&body(&format!("```{info}\n<b/>\n```")))
				.xpect_eq(vec!["0", "0/b.0"]);
		}
	}

	/// A format taking no params refuses one, and `render` is a flag.
	#[beet_core::test]
	fn a_rendered_fences_params_are_checked() {
		for info in ["html render title=\"a\"", "html render=yes"] {
			ScenePlugin::parsers()
				.body(&MediaBytes::new_markdown(&format!(
					"```{info}\n<b/>\n```"
				)))
				.xpect_err();
		}
	}

	/// A fence's document that does not parse refuses the page.
	#[beet_core::test]
	fn a_malformed_fence_refuses_its_page() {
		ScenePlugin::parsers()
			.body(&MediaBytes::new_markdown(
				"```application/vnd.beet.bsx render\n<a>{@nope:x}</a>\n```",
			))
			.xpect_err();
	}
}
