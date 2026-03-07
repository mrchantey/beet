//! Streaming HTML parser that diffs parsed content against an existing entity tree.
//!
//! This module implements [`NodeParser`] for HTML, using `winnow` parser
//! combinators to tokenize the input and a depth-first differ to apply
//! minimal ECS mutations.
//!
//! Enable with the `html_parser` feature flag.

pub(crate) mod combinators;
pub(crate) mod diff;
pub(crate) mod tokens;

pub use combinators::HtmlParseConfig;
pub use diff::*;
pub use tokens::*;

use crate::prelude::*;
use beet_core::prelude::*;

/// A configurable HTML parser that implements [`NodeParser`].
///
/// Parses HTML into a tree of ECS entities, diffing against any existing
/// hierarchy to avoid unnecessary mutations. Supports expressions,
/// void elements, raw text elements, and configurable error handling.
///
/// Configuration is split into two internal structs:
/// - [`ParseConfig`] controls tokenization behavior (expressions, raw text elements).
/// - [`DiffConfig`] controls entity diffing behavior (value parsing, void elements,
///   error handling).
#[derive(Debug, Clone)]
pub struct HtmlParser {
	/// Tokenization configuration.
	pub parse_config: HtmlParseConfig,
	/// Entity diffing configuration.
	pub diff_config: HtmlDiffConfig,
	/// When enabled, text nodes are re-parsed as markdown after the
	/// HTML tree is built. Requires the `markdown_parser` feature.
	#[cfg(feature = "markdown_parser")]
	pub parse_markdown: Option<MarkdownParseConfig>,
}

impl Default for HtmlParser {
	fn default() -> Self {
		Self {
			parse_config: HtmlParseConfig::default(),
			diff_config: HtmlDiffConfig::default(),
			#[cfg(feature = "markdown_parser")]
			parse_markdown: None,
		}
	}
}

impl HtmlParser {
	/// Create a new parser with default HTML5 settings.
	pub fn new() -> Self { Self::default() }

	/// Create a parser with expression support enabled.
	pub fn with_expressions() -> Self {
		Self {
			parse_config: HtmlParseConfig {
				parse_expressions: true,
				parse_raw_text_expressions: true,
				..default()
			},
			diff_config: default(),
			#[cfg(feature = "markdown_parser")]
			parse_markdown: None,
		}
	}

	/// Enable parsing text nodes as markdown.
	///
	/// When enabled, after building the HTML entity tree, each text
	/// node's content is re-parsed as markdown and the resulting
	/// subtree replaces the original text node. Requires the
	/// `markdown_parser` feature.
	#[cfg(feature = "markdown_parser")]
	pub fn with_markdown(mut self) -> Self {
		self.parse_markdown = Some(default());
		self
	}

	/// Shared parsing logic: tokenize, build tree, diff against entity.
	fn parse_text(
		&self,
		world: &mut World,
		entity: Entity,
		text: &str,
		path: Option<&WsPathBuf>,
	) -> Result {
		// tokenize
		let tokens = combinators::parse_document(text, &self.parse_config)?;

		// build tree from flat tokens
		let tree =
			build_html_tree(&tokens, &self.diff_config, &self.parse_config)?;

		// build span lookup if path was provided
		let span_lookup = path.map(|path| SpanLookup::new(text, path.clone()));

		// diff tree against entity, note the root is not a node so is not diffed
		diff_children(
			world,
			entity,
			&tree,
			&self.diff_config,
			span_lookup.as_ref(),
		)?;

		// if markdown parsing is enabled, re-parse text node
		// children as markdown subtrees
		#[cfg(feature = "markdown_parser")]
		if let Some(ref md_config) = self.parse_markdown {
			reparse_text_nodes_as_markdown(
				world,
				entity,
				&self.diff_config,
				md_config,
				span_lookup.as_ref(),
			)?;
		}

		// insert file span on the root entity if path provided
		if let Some(ref lookup) = span_lookup {
			let span = lookup.full_span();
			world.entity_mut(entity).set_if_ne_or_insert(span);
		}

		Ok(())
	}
}

impl NodeParser for HtmlParser {
	fn parse(
		&mut self,
		world: &mut World,
		entity: Entity,
		bytes: Vec<u8>,
		path: Option<WsPathBuf>,
	) -> Result {
		let text = std::str::from_utf8(&bytes)?;
		self.parse_text(world, entity, text, path.as_ref())
	}
}

/// Recursively walk the entity tree rooted at `parent`, find text-only
/// child entities (those with [`Value`] but no [`Element`]), re-parse
/// their content as markdown, and replace them with the resulting subtree.
#[cfg(feature = "markdown_parser")]
fn reparse_text_nodes_as_markdown(
	world: &mut World,
	parent: Entity,
	diff_config: &HtmlDiffConfig,
	md_config: &MarkdownParseConfig,
	span_lookup: Option<&SpanLookup>,
) -> Result {
	use crate::parse::html::diff::HtmlNode;
	use crate::parse::html::diff::spawn_node;

	let children: Vec<Entity> = world
		.entity(parent)
		.get::<Children>()
		.map(|children| children.iter().collect())
		.unwrap_or_default();

	for child in children {
		let entity_ref = world.entity(child);
		let has_element = entity_ref.get::<Element>().is_some();
		let text_value = entity_ref.get::<Value>().cloned();

		if has_element {
			// recurse into element children
			reparse_text_nodes_as_markdown(
				world,
				child,
				diff_config,
				md_config,
				span_lookup,
			)?;
		}
		if let Some(Value::Str(ref text)) = text_value {
			if text.trim().is_empty() {
				continue;
			}
			// try to parse as markdown
			let parse_config =
				crate::parse::html::combinators::HtmlParseConfig::default();
			let md_result = build_markdown_tree(
				text,
				md_config.options,
				&parse_config,
				diff_config,
				None,
			)?;

			// only replace if markdown produced structure beyond a
			// single text node (ie actual markdown formatting)
			let dominated_by_single_text = md_result.nodes.len() == 1
				&& matches!(md_result.nodes[0], HtmlNode::Text(_));

			if !dominated_by_single_text && !md_result.nodes.is_empty() {
				// replace the text entity with the markdown subtree
				// remove the Value component and insert children
				world.entity_mut(child).remove::<Value>();

				for node in &md_result.nodes {
					spawn_node(world, child, node, diff_config, span_lookup)?;
				}
			}
		}
	}
	Ok(())
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Collect the entity ids of the direct children.
	fn get_children(world: &World, entity: Entity) -> Vec<Entity> {
		world
			.entity(entity)
			.get::<Children>()
			.map(|children| children.iter().collect())
			.unwrap_or_default()
	}

	#[test]
	fn parse_simple_element() {
		let mut world = World::new();
		let entity = world.spawn(()).id();
		HtmlParser::new()
			.parse(&mut world, entity, b"<div>hello</div>".to_vec(), None)
			.unwrap();
		get_children(&world, entity).len().xpect_eq(1);
	}

	#[test]
	fn parse_text_node() {
		let mut world = World::new();
		let entity = world.spawn(()).id();
		HtmlParser::new()
			.parse(&mut world, entity, b"hello world".to_vec(), None)
			.unwrap();
		let children = get_children(&world, entity);
		world
			.entity(children[0])
			.get::<Value>()
			.cloned()
			.unwrap()
			.xpect_eq(Value::Str("hello world".into()));
	}

	#[test]
	fn parse_nested_elements() {
		let mut world = World::new();
		let entity = world.spawn(()).id();
		HtmlParser::new()
			.parse(
				&mut world,
				entity,
				b"<div><span>inner</span></div>".to_vec(),
				None,
			)
			.unwrap();
		let root_children = get_children(&world, entity);
		let div = root_children[0];
		let div_children = get_children(&world, div);
		let span = div_children[0];
		world
			.entity(span)
			.get::<Element>()
			.unwrap()
			.name()
			.to_string()
			.xpect_eq("span".to_string());
	}

	#[test]
	fn parse_with_expressions() {
		let mut world = World::new();
		let entity = world.spawn(()).id();
		HtmlParser::with_expressions()
			.parse(&mut world, entity, b"<p>hello {name}</p>".to_vec(), None)
			.unwrap();
		let root_children = get_children(&world, entity);
		let p_children = get_children(&world, root_children[0]);
		world
			.entity(p_children[1])
			.get::<Expression>()
			.unwrap()
			.0
			.clone()
			.xpect_eq("name".to_string());
	}

	#[test]
	fn parse_void_element() {
		let mut world = World::new();
		let entity = world.spawn(()).id();
		HtmlParser::new()
			.parse(&mut world, entity, b"<div><br>text</div>".to_vec(), None)
			.unwrap();
		let root_children = get_children(&world, entity);
		let div_children = get_children(&world, root_children[0]);
		world
			.entity(div_children[0])
			.get::<Element>()
			.unwrap()
			.name()
			.to_string()
			.xpect_eq("br".to_string());
	}

	#[test]
	fn parse_with_path_inserts_file_span() {
		let mut world = World::new();
		let entity = world.spawn(()).id();
		HtmlParser::new()
			.parse(
				&mut world,
				entity,
				b"<div>hello</div>".to_vec(),
				Some(WsPathBuf::new("test.html")),
			)
			.unwrap();
		world
			.entity(entity)
			.get::<FileSpan>()
			.cloned()
			.unwrap()
			.path()
			.xpect_eq(WsPathBuf::new("test.html"));
	}

	#[test]
	fn parse_comment() {
		let mut world = World::new();
		let entity = world.spawn(()).id();
		HtmlParser::new()
			.parse(&mut world, entity, b"<!-- hello -->".to_vec(), None)
			.unwrap();
		let root_children = get_children(&world, entity);
		world
			.entity(root_children[0])
			.get::<Comment>()
			.cloned()
			.unwrap()
			.xpect_eq(Comment::new(" hello "));
	}

	#[test]
	fn parse_value_parsing_enabled() {
		let mut world = World::new();
		let entity = world.spawn(()).id();
		let mut parser = HtmlParser {
			diff_config: HtmlDiffConfig {
				parse_text_nodes: true,
				..default()
			},
			..default()
		};
		parser
			.parse(&mut world, entity, b"<div>42</div>".to_vec(), None)
			.unwrap();
		let root_children = get_children(&world, entity);
		let div_children = get_children(&world, root_children[0]);
		world
			.entity(div_children[0])
			.get::<Value>()
			.cloned()
			.unwrap()
			.xpect_eq(Value::Uint(42));
	}

	#[test]
	fn parse_attributes() {
		let mut world = World::new();
		let entity = world.spawn(()).id();
		HtmlParser::new()
			.parse(
				&mut world,
				entity,
				b"<div class=\"foo\" id=\"bar\"></div>".to_vec(),
				None,
			)
			.unwrap();
		let root_children = get_children(&world, entity);
		let div = root_children[0];
		let attrs = world
			.entity(div)
			.get::<Attributes>()
			.map(|attrs| {
				let mut result = Vec::new();
				for attr_entity in attrs.iter() {
					let attr_ref = world.entity(attr_entity);
					let key = attr_ref.get::<Attribute>().unwrap().to_string();
					let val =
						attr_ref.get::<Value>().cloned().unwrap_or_default();
					result.push((key, val));
				}
				result
			})
			.unwrap_or_default();
		attrs.len().xpect_eq(2);
	}

	#[test]
	fn parse_self_closing() {
		let mut world = World::new();
		let entity = world.spawn(()).id();
		HtmlParser::new()
			.parse(&mut world, entity, b"<img />".to_vec(), None)
			.unwrap();
		get_children(&world, entity).len().xpect_eq(1);
	}

	#[test]
	fn reparse_unchanged_no_change() {
		let mut world = World::new();
		let entity = world.spawn(()).id();
		let mut parser = HtmlParser::new();
		let html = b"<div>hello</div>".to_vec();
		parser
			.parse(&mut world, entity, html.clone(), None)
			.unwrap();
		parser.parse(&mut world, entity, html, None).unwrap();
		get_children(&world, entity).len().xpect_eq(1);
	}

	#[test]
	fn reparse_changed_content() {
		let mut world = World::new();
		let entity = world.spawn(()).id();
		let mut parser = HtmlParser::new();
		parser
			.parse(&mut world, entity, b"<div>hello</div>".to_vec(), None)
			.unwrap();
		parser
			.parse(&mut world, entity, b"<div>world</div>".to_vec(), None)
			.unwrap();
		let root_children = get_children(&world, entity);
		let div_children = get_children(&world, root_children[0]);
		world
			.entity(div_children[0])
			.get::<Value>()
			.cloned()
			.unwrap()
			.xpect_eq(Value::Str("world".into()));
	}

	#[test]
	fn element_span_covers_opening_tag() {
		let mut world = World::new();
		let entity = world.spawn(()).id();
		HtmlParser::new()
			.parse(
				&mut world,
				entity,
				b"<div>hello</div>".to_vec(),
				Some(WsPathBuf::new("test.html")),
			)
			.unwrap();
		let root_children = get_children(&world, entity);
		world
			.entity(root_children[0])
			.get::<FileSpan>()
			.cloned()
			.unwrap()
			.xpect_eq(FileSpan::new(
				"test.html",
				LineCol::new(1, 0),
				LineCol::new(1, 5),
			));
	}

	#[test]
	fn text_node_span() {
		let mut world = World::new();
		let entity = world.spawn(()).id();
		HtmlParser::new()
			.parse(
				&mut world,
				entity,
				b"<div>hello</div>".to_vec(),
				Some(WsPathBuf::new("test.html")),
			)
			.unwrap();
		let root_children = get_children(&world, entity);
		let div_children = get_children(&world, root_children[0]);
		world
			.entity(div_children[0])
			.get::<FileSpan>()
			.cloned()
			.unwrap()
			.xpect_eq(FileSpan::new(
				"test.html",
				LineCol::new(1, 5),
				LineCol::new(1, 10),
			));
	}

	#[test]
	fn multiline_spans() {
		let mut world = World::new();
		let entity = world.spawn(()).id();
		// line 1: <div>\n
		// line 2: hello\n
		// line 3: </div>
		HtmlParser::new()
			.parse(
				&mut world,
				entity,
				b"<div>\nhello\n</div>".to_vec(),
				Some(WsPathBuf::new("test.html")),
			)
			.unwrap();
		let root_children = get_children(&world, entity);
		let div_children = get_children(&world, root_children[0]);
		world
			.entity(div_children[0])
			.get::<FileSpan>()
			.cloned()
			.unwrap()
			.xpect_eq(FileSpan::new(
				"test.html",
				LineCol::new(1, 5), // after `<div>`
				LineCol::new(3, 0), // up to start of `</div>`
			));
	}

	#[test]
	fn attribute_entity_has_span() {
		let mut world = World::new();
		let entity = world.spawn(()).id();
		HtmlParser::new()
			.parse(
				&mut world,
				entity,
				b"<div class=\"foo\"></div>".to_vec(),
				Some(WsPathBuf::new("test.html")),
			)
			.unwrap();
		let root_children = get_children(&world, entity);
		let div = root_children[0];
		let attrs = world.entity(div).get::<Attributes>().unwrap();
		let attr_entity = attrs.iter().next().unwrap();
		world
			.entity(attr_entity)
			.get::<FileSpan>()
			.cloned()
			.unwrap()
			.xpect_eq(FileSpan::new(
				"test.html",
				// span covers `class` through `foo` (key offset to value end)
				LineCol::new(1, 5),
				LineCol::new(1, 15),
			));
	}

	#[test]
	fn expression_node_span() {
		let mut world = World::new();
		let entity = world.spawn(()).id();
		HtmlParser::with_expressions()
			.parse(
				&mut world,
				entity,
				b"<p>{name}</p>".to_vec(),
				Some(WsPathBuf::new("test.html")),
			)
			.unwrap();
		let root_children = get_children(&world, entity);
		let p_children = get_children(&world, root_children[0]);
		world
			.entity(p_children[0])
			.get::<FileSpan>()
			.cloned()
			.unwrap()
			.xpect_eq(FileSpan::new(
				"test.html",
				// span covers `name` (the expression content inside braces)
				LineCol::new(1, 4),
				LineCol::new(1, 8),
			));
	}

	#[cfg(feature = "markdown_parser")]
	#[test]
	fn parse_markdown_text_nodes() {
		let mut world = World::new();
		let entity = world.spawn(()).id();
		// The div contains markdown text "**bold**" which should
		// be re-parsed into <p><strong>bold</strong></p>
		HtmlParser::new()
			.with_markdown()
			.parse(&mut world, entity, b"<div>**bold**</div>".to_vec(), None)
			.unwrap();
		let root_children = get_children(&world, entity);
		let div_children = get_children(&world, root_children[0]);
		// the text node should now be replaced with a subtree
		// containing a <p> with <strong>
		world
			.entity(div_children[0])
			.get::<Children>()
			.is_some()
			.xpect_true();
	}

	#[cfg(feature = "markdown_parser")]
	#[test]
	fn parse_markdown_preserves_plain_text() {
		let mut world = World::new();
		let entity = world.spawn(()).id();
		// Plain text without markdown formatting should still
		// get wrapped in a <p> element by the markdown parser
		HtmlParser::new()
			.with_markdown()
			.parse(&mut world, entity, b"<div>hello world</div>".to_vec(), None)
			.unwrap();
		let root_children = get_children(&world, entity);
		let div_children = get_children(&world, root_children[0]);
		// "hello world" parsed as markdown becomes <p>hello world</p>
		// so the original text node is replaced with structure
		div_children.len().xpect_eq(1);
	}
}
