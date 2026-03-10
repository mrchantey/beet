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
	fn parse(&mut self, cx: ParseContext) -> Result<(), ParseError> {
		let media_type = cx.bytes.media_type();
		if *media_type != MediaType::Html {
			return Err(ParseError::UnsupportedType {
				unsupported: media_type.clone(),
				supported: vec![MediaType::Html],
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

	/// Parse HTML bytes into a freshly spawned entity, returning the entity ref.
	fn parse_html(entity: &mut EntityWorldMut<'_>, html: &str) {
		let bytes = MediaBytes::html(html);
		HtmlParser::new()
			.parse(ParseContext::new(entity, &bytes))
			.unwrap();
	}

	/// Parse HTML with a path for span tracking.
	fn parse_html_with_path(
		entity: &mut EntityWorldMut<'_>,
		html: &str,
		path: &str,
	) {
		let bytes = MediaBytes::html(html);
		HtmlParser::new()
			.parse(
				ParseContext::new(entity, &bytes)
					.with_path(WsPathBuf::new(path)),
			)
			.unwrap();
	}

	#[test]
	fn parse_simple_element() {
		World::new()
			.spawn_empty()
			.xtap(|entity| parse_html(entity, "<div>hello</div>"))
			.children()
			.len()
			.xpect_eq(1);
	}

	#[test]
	fn parse_text_node() {
		World::new()
			.spawn_empty()
			.xtap(|entity| parse_html(entity, "hello world"))
			.child(0)
			.unwrap()
			.get::<Value>()
			.cloned()
			.unwrap()
			.xpect_eq(Value::Str("hello world".into()));
	}

	#[test]
	fn parse_nested_elements() {
		World::new()
			.spawn_empty()
			.xtap(|entity| parse_html(entity, "<div><span>inner</span></div>"))
			// root -> div -> span
			.child(0)
			.unwrap()
			.child(0)
			.unwrap()
			.get::<Element>()
			.unwrap()
			.name()
			.to_string()
			.xpect_eq("span".to_string());
	}

	#[test]
	fn parse_with_expressions() {
		let bytes = MediaBytes::html("<p>hello {name}</p>");
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				HtmlParser::with_expressions()
					.parse(ParseContext::new(entity, &bytes))
					.unwrap();
			})
			// root -> p -> expression (index 1, after the "hello " text node)
			.child(0)
			.unwrap()
			.child(1)
			.unwrap()
			.get::<Expression>()
			.unwrap()
			.0
			.clone()
			.xpect_eq("name".to_string());
	}

	#[test]
	fn parse_void_element() {
		World::new()
			.spawn_empty()
			.xtap(|entity| parse_html(entity, "<div><br>text</div>"))
			// root -> div -> br
			.child(0)
			.unwrap()
			.child(0)
			.unwrap()
			.get::<Element>()
			.unwrap()
			.name()
			.to_string()
			.xpect_eq("br".to_string());
	}

	#[test]
	fn parse_with_path_inserts_file_span() {
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				parse_html_with_path(entity, "<div>hello</div>", "test.html")
			})
			.get::<FileSpan>()
			.cloned()
			.unwrap()
			.path()
			.xpect_eq(WsPathBuf::new("test.html"));
	}

	#[test]
	fn parse_comment() {
		World::new()
			.spawn_empty()
			.xtap(|entity| parse_html(entity, "<!-- hello -->"))
			.child(0)
			.unwrap()
			.get::<Comment>()
			.cloned()
			.unwrap()
			.xpect_eq(Comment::new(" hello "));
	}

	#[test]
	fn parse_value_parsing_enabled() {
		let mut parser = HtmlParser {
			diff_config: HtmlDiffConfig {
				parse_text_nodes: true,
				..default()
			},
			..default()
		};
		let bytes = MediaBytes::html("<div>42</div>");
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				parser.parse(ParseContext::new(entity, &bytes)).unwrap();
			})
			// root -> div -> text node
			.child(0)
			.unwrap()
			.child(0)
			.unwrap()
			.get::<Value>()
			.cloned()
			.unwrap()
			.xpect_eq(Value::Uint(42));
	}

	#[test]
	fn parse_attributes() {
		let mut world = World::new();
		let entity = world
			.spawn_empty()
			.xtap(|entity| {
				parse_html(entity, "<div class=\"foo\" id=\"bar\"></div>")
			})
			.id();
		let div = world.entity_mut(entity).child(0).unwrap().id();
		world
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
			.unwrap_or_default()
			.len()
			.xpect_eq(2);
	}

	#[test]
	fn parse_self_closing() {
		World::new()
			.spawn_empty()
			.xtap(|entity| parse_html(entity, "<img />"))
			.children()
			.len()
			.xpect_eq(1);
	}

	#[test]
	fn reparse_unchanged_no_change() {
		let bytes = MediaBytes::html("<div>hello</div>");
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				let mut parser = HtmlParser::new();
				parser.parse(ParseContext::new(entity, &bytes)).unwrap();
				parser.parse(ParseContext::new(entity, &bytes)).unwrap();
			})
			.children()
			.len()
			.xpect_eq(1);
	}

	#[test]
	fn reparse_changed_content() {
		let bytes_a = MediaBytes::html("<div>hello</div>");
		let bytes_b = MediaBytes::html("<div>world</div>");
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				let mut parser = HtmlParser::new();
				parser.parse(ParseContext::new(entity, &bytes_a)).unwrap();
				parser.parse(ParseContext::new(entity, &bytes_b)).unwrap();
			})
			// root -> div -> text node
			.child(0)
			.unwrap()
			.child(0)
			.unwrap()
			.get::<Value>()
			.cloned()
			.unwrap()
			.xpect_eq(Value::Str("world".into()));
	}

	#[test]
	fn element_span_covers_opening_tag() {
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				parse_html_with_path(entity, "<div>hello</div>", "test.html")
			})
			.child(0)
			.unwrap()
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
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				parse_html_with_path(entity, "<div>hello</div>", "test.html")
			})
			// root -> div -> text node
			.child(0)
			.unwrap()
			.child(0)
			.unwrap()
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
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				// line 1: <div>\n
				// line 2: hello\n
				// line 3: </div>
				parse_html_with_path(
					entity,
					"<div>\nhello\n</div>",
					"test.html",
				)
			})
			// root -> div -> text node
			.child(0)
			.unwrap()
			.child(0)
			.unwrap()
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
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				parse_html_with_path(
					entity,
					"<div class=\"foo\"></div>",
					"test.html",
				)
			})
			.child(0)
			.unwrap()
			.related::<Attributes>()[0]
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
		let bytes = MediaBytes::html("<p>{name}</p>");
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				HtmlParser::with_expressions()
					.parse(
						ParseContext::new(entity, &bytes)
							.with_path(WsPathBuf::new("test.html")),
					)
					.unwrap();
			})
			// root -> p -> expression node
			.child(0)
			.unwrap()
			.child(0)
			.unwrap()
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
		// The div contains markdown text "**bold**" which should
		// be re-parsed into <p><strong>bold</strong></p>
		let bytes = MediaBytes::html("<div>**bold**</div>");
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				HtmlParser::new()
					.with_markdown()
					.parse(ParseContext::new(entity, &bytes))
					.unwrap();
			})
			// root -> div -> (replaced text node with subtree)
			.child(0)
			.unwrap()
			.child(0)
			.unwrap()
			.get::<Children>()
			.is_some()
			.xpect_true();
	}

	#[cfg(feature = "markdown_parser")]
	#[test]
	fn parse_markdown_preserves_plain_text() {
		// Plain text without markdown formatting should still
		// get wrapped in a <p> element by the markdown parser.
		// "hello world" parsed as markdown becomes <p>hello world</p>
		// so the original text node is replaced with structure.
		let bytes = MediaBytes::html("<div>hello world</div>");
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				HtmlParser::new()
					.with_markdown()
					.parse(ParseContext::new(entity, &bytes))
					.unwrap();
			})
			// root -> div -> children count
			.child(0)
			.unwrap()
			.children()
			.len()
			.xpect_eq(1);
	}
}
