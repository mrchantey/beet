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

pub use combinators::ParseConfig;
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
	pub parse_config: ParseConfig,
	/// Entity diffing configuration.
	pub diff_config: DiffConfig,
	/// When enabled, text nodes are re-parsed as markdown after the
	/// HTML tree is built. Requires the `markdown_parser` feature.
	#[cfg(feature = "markdown_parser")]
	pub parse_markdown: bool,
}

impl Default for HtmlParser {
	fn default() -> Self {
		Self {
			parse_config: ParseConfig::default(),
			diff_config: DiffConfig::default(),
			#[cfg(feature = "markdown_parser")]
			parse_markdown: false,
		}
	}
}

impl HtmlParser {
	/// Create a new parser with default HTML5 settings.
	pub fn new() -> Self { Self::default() }

	/// Create a parser with expression support enabled.
	pub fn with_expressions() -> Self {
		Self {
			parse_config: ParseConfig {
				parse_expressions: true,
				parse_raw_text_expressions: true,
				..Default::default()
			},
			diff_config: DiffConfig::default(),
			#[cfg(feature = "markdown_parser")]
			parse_markdown: false,
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
		self.parse_markdown = true;
		self
	}

	/// Shared parsing logic: tokenize, build tree, diff against entity.
	///
	/// This is the core implementation used by both [`NodeParser::parse`]
	/// and the streaming path. All work happens synchronously inside a
	/// single world access.
	async fn parse_text(
		&self,
		entity: &AsyncEntity,
		text: &str,
		path: Option<&WsPathBuf>,
	) -> Result {
		let parse_config = self.parse_config.clone();
		let diff_config = self.diff_config.clone();
		#[cfg(feature = "markdown_parser")]
		let parse_markdown = self.parse_markdown;
		let id = entity.id();
		let text_owned = text.to_string();
		let path_owned = path.cloned();

		entity
			.world()
			.with_then(move |world| -> Result {
				// tokenize
				let tokens =
					combinators::parse_document(&text_owned, &parse_config)?;

				// build tree from flat tokens
				let tree = build_tree(&tokens, &diff_config, &parse_config)?;

				// build span lookup if path was provided
				let span_lookup = path_owned
					.as_ref()
					.map(|path| SpanLookup::new(&text_owned, path.clone()));

				// diff tree against entity, note the root is not a node so is not diffed
				diff_children(
					world,
					id,
					&tree,
					&diff_config,
					span_lookup.as_ref(),
				)?;

				// if markdown parsing is enabled, re-parse text node
				// children as markdown subtrees
				#[cfg(feature = "markdown_parser")]
				if parse_markdown {
					reparse_text_nodes_as_markdown(
						world,
						id,
						&diff_config,
						span_lookup.as_ref(),
					)?;
				}

				// insert file span on the root entity if path provided
				if let Some(ref lookup) = span_lookup {
					let span = lookup.full_span();
					world.entity_mut(id).set_if_ne_or_insert(span);
				}

				Ok(())
			})
			.await?;

		Ok(())
	}
}

impl NodeParser for HtmlParser {
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

/// Recursively walk the entity tree rooted at `parent`, find text-only
/// child entities (those with [`Value`] but no [`Element`]), re-parse
/// their content as markdown, and replace them with the resulting subtree.
#[cfg(feature = "markdown_parser")]
fn reparse_text_nodes_as_markdown(
	world: &mut World,
	parent: Entity,
	diff_config: &DiffConfig,
	span_lookup: Option<&SpanLookup>,
) -> Result {
	use crate::parse::html::diff::TreeNode;
	use crate::parse::html::diff::spawn_node;
	use crate::parse::markdown::tree_builder;

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
				span_lookup,
			)?;
		}
		if let Some(Value::Str(ref text)) = text_value {
			if text.trim().is_empty() {
				continue;
			}
			// try to parse as markdown
			let parse_config =
				crate::parse::html::combinators::ParseConfig::default();
			let md_result = tree_builder::build_markdown_tree(
				text,
				crate::prelude::MarkdownParser::default_options(),
				&parse_config,
				diff_config,
				None,
			)?;

			// only replace if markdown produced structure beyond a
			// single text node (ie actual markdown formatting)
			let dominated_by_single_text = md_result.nodes.len() == 1
				&& matches!(md_result.nodes[0], TreeNode::Text(_));

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
	async fn parse_simple_element() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				HtmlParser::new()
					.parse(entity, b"<div>hello</div>".to_vec(), None)
					.await
					.unwrap();
				// root entity should have one child: the div
				let children = get_children(&entity).await;
				children.len()
			})
			.await
			.xpect_eq(1);
	}

	#[beet_core::test]
	async fn parse_text_node() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				HtmlParser::new()
					.parse(entity, b"hello world".to_vec(), None)
					.await
					.unwrap();
				let children = get_children(&entity).await;
				// should have one text child
				let child = world.entity(children[0]);
				child.get_cloned::<Value>().await.unwrap()
			})
			.await
			.xpect_eq(Value::Str("hello world".into()));
	}

	#[beet_core::test]
	async fn parse_nested_elements() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				HtmlParser::new()
					.parse(
						entity,
						b"<div><span>inner</span></div>".to_vec(),
						None,
					)
					.await
					.unwrap();
				// root -> div -> span -> "inner"
				let root_children = get_children(&entity).await;

				let div = world.entity(root_children[0]);
				let div_children = get_children(&div).await;

				let span = world.entity(div_children[0]);
				let span_children = get_children(&span).await;

				let text_entity = world.entity(span_children[0]);
				text_entity.get_cloned::<Value>().await.unwrap()
			})
			.await
			.xpect_eq(Value::Str("inner".into()));
	}

	#[beet_core::test]
	async fn parse_with_expressions() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				HtmlParser::with_expressions()
					.parse(entity, b"<p>hello {name}</p>".to_vec(), None)
					.await
					.unwrap();
				// p should have two children: text "hello " and expression "name"
				let root_children = get_children(&entity).await;

				let p_entity = world.entity(root_children[0]);
				let p_children = get_children(&p_entity).await;

				p_children.len()
			})
			.await
			.xpect_eq(2);
	}

	#[beet_core::test]
	async fn parse_void_element() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				HtmlParser::new()
					.parse(entity, b"<div><br>text</div>".to_vec(), None)
					.await
					.unwrap();
				// div should have 2 children: br (no children) and text
				let root_children = get_children(&entity).await;

				let div = world.entity(root_children[0]);
				let div_children = get_children(&div).await;

				div_children.len()
			})
			.await
			.xpect_eq(2);
	}

	#[beet_core::test]
	async fn parse_with_path_inserts_file_span() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				HtmlParser::new()
					.parse(
						entity,
						b"<div>hello</div>".to_vec(),
						Some(WsPathBuf::new("test.html")),
					)
					.await
					.unwrap();
				entity.get_cloned::<FileSpan>().await.unwrap()
			})
			.await
			.path()
			.xpect_eq(WsPathBuf::new("test.html"));
	}

	#[beet_core::test]
	async fn parse_comment() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				HtmlParser::new()
					.parse(entity, b"<!-- hello -->".to_vec(), None)
					.await
					.unwrap();
				let children = get_children(&entity).await;

				let child = world.entity(children[0]);
				let comment: Comment = child
					.with_then(|entity| {
						entity.get::<Comment>().cloned().unwrap()
					})
					.await;

				comment
			})
			.await
			.xpect_eq(Comment::new(" hello "));
	}

	#[beet_core::test]
	async fn parse_value_parsing_enabled() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				let mut parser = HtmlParser {
					diff_config: DiffConfig {
						parse_text_nodes: true,
						..Default::default()
					},
					..Default::default()
				};
				parser
					.parse(entity, b"<div>42</div>".to_vec(), None)
					.await
					.unwrap();

				let root_children = get_children(&entity).await;

				let div = world.entity(root_children[0]);
				let div_children = get_children(&div).await;

				let text = world.entity(div_children[0]);
				text.get_cloned::<Value>().await.unwrap()
			})
			.await
			.xpect_eq(Value::Uint(42));
	}

	#[beet_core::test]
	async fn parse_attributes() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				HtmlParser::new()
					.parse(
						entity,
						b"<div class=\"foo\" id=\"bar\"></div>".to_vec(),
						None,
					)
					.await
					.unwrap();
				let root_children = get_children(&entity).await;
				let div = world.entity(root_children[0]);

				// collect attribute values
				let attrs: Vec<(String, Value)> = div
					.with_then(|entity| {
						entity
							.get::<Attributes>()
							.map(|attrs| {
								let mut result = Vec::new();
								for attr_entity in attrs.iter() {
									let attr_ref =
										entity.world().entity(attr_entity);
									let key = attr_ref
										.get::<Attribute>()
										.unwrap()
										.to_string();
									let val = attr_ref
										.get::<Value>()
										.cloned()
										.unwrap_or_default();
									result.push((key, val));
								}
								result
							})
							.unwrap_or_default()
					})
					.await;

				attrs.len()
			})
			.await
			.xpect_eq(2);
	}

	#[beet_core::test]
	async fn parse_self_closing() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				HtmlParser::new()
					.parse(entity, b"<img />".to_vec(), None)
					.await
					.unwrap();
				let children = get_children(&entity).await;
				children.len()
			})
			.await
			.xpect_eq(1);
	}

	#[beet_core::test]
	async fn parse_stream_collects_and_parses() {
		use bevy::tasks::futures_lite::stream;

		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				let chunks: Vec<Result<Vec<u8>>> = vec![
					Ok(b"<div>".to_vec()),
					Ok(b"hello".to_vec()),
					Ok(b"</div>".to_vec()),
				];
				HtmlParser::new()
					.parse_stream(entity, stream::iter(chunks), None)
					.await
					.unwrap();
				let children = get_children(&entity).await;
				children.len()
			})
			.await
			.xpect_eq(1);
	}

	#[beet_core::test]
	async fn reparse_unchanged_no_change() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				let mut parser = HtmlParser::new();
				let html = b"<div>hello</div>".to_vec();
				parser.parse(entity, html.clone(), None).await.unwrap();
				// parse again with same content
				parser.parse(entity, html, None).await.unwrap();
				let children = get_children(&entity).await;
				children.len()
			})
			.await
			.xpect_eq(1);
	}

	#[beet_core::test]
	async fn reparse_changed_content() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				let mut parser = HtmlParser::new();
				parser
					.parse(entity, b"<div>hello</div>".to_vec(), None)
					.await
					.unwrap();
				// change content
				parser
					.parse(entity, b"<div>world</div>".to_vec(), None)
					.await
					.unwrap();
				let root_children = get_children(&entity).await;
				let div = world.entity(root_children[0]);
				let div_children = get_children(&div).await;
				let text = world.entity(div_children[0]);
				text.get_cloned::<Value>().await.unwrap()
			})
			.await
			.xpect_eq(Value::Str("world".into()));
	}

	#[beet_core::test]
	async fn element_span_covers_opening_tag() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				HtmlParser::new()
					.parse(
						entity,
						b"<div>hello</div>".to_vec(),
						Some(WsPathBuf::new("test.html")),
					)
					.await
					.unwrap();
				let root_children = get_children(&entity).await;
				let div = world.entity(root_children[0]);
				div.get_cloned::<FileSpan>().await.unwrap()
			})
			.await
			.xpect_eq(FileSpan::new(
				"test.html",
				LineCol::new(1, 0),
				LineCol::new(1, 5),
			));
	}

	#[beet_core::test]
	async fn text_node_span() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				HtmlParser::new()
					.parse(
						entity,
						b"<div>hello</div>".to_vec(),
						Some(WsPathBuf::new("test.html")),
					)
					.await
					.unwrap();
				let root_children = get_children(&entity).await;
				let div = world.entity(root_children[0]);
				let div_children = get_children(&div).await;
				let text = world.entity(div_children[0]);
				text.get_cloned::<FileSpan>().await.unwrap()
			})
			.await
			.xpect_eq(FileSpan::new(
				"test.html",
				LineCol::new(1, 5),
				LineCol::new(1, 10),
			));
	}

	#[beet_core::test]
	async fn multiline_spans() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				// line 1: <div>\n
				// line 2: hello\n
				// line 3: </div>
				HtmlParser::new()
					.parse(
						entity,
						b"<div>\nhello\n</div>".to_vec(),
						Some(WsPathBuf::new("test.html")),
					)
					.await
					.unwrap();
				let root_children = get_children(&entity).await;
				let div = world.entity(root_children[0]);
				let div_children = get_children(&div).await;
				let text = world.entity(div_children[0]);
				text.get_cloned::<FileSpan>().await.unwrap()
			})
			.await
			.xpect_eq(FileSpan::new(
				"test.html",
				LineCol::new(1, 5), // after `<div>`
				LineCol::new(3, 0), // up to start of `</div>`
			));
	}

	#[beet_core::test]
	async fn attribute_entity_has_span() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				HtmlParser::new()
					.parse(
						entity,
						b"<div class=\"foo\"></div>".to_vec(),
						Some(WsPathBuf::new("test.html")),
					)
					.await
					.unwrap();
				let root_children = get_children(&entity).await;
				let div = world.entity(root_children[0]);

				// get the attribute entity
				let attr_span: FileSpan = div
					.with_then(|entity| {
						let attrs = entity.get::<Attributes>().unwrap();
						let attr_entity = attrs.iter().next().unwrap();
						entity
							.world()
							.entity(attr_entity)
							.get::<FileSpan>()
							.cloned()
							.unwrap()
					})
					.await;

				attr_span
			})
			.await
			.xpect_eq(FileSpan::new(
				"test.html",
				// span covers `class` through `foo` (key offset to value end)
				LineCol::new(1, 5),
				LineCol::new(1, 15),
			));
	}

	#[beet_core::test]
	async fn expression_node_span() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				HtmlParser::with_expressions()
					.parse(
						entity,
						b"<p>{name}</p>".to_vec(),
						Some(WsPathBuf::new("test.html")),
					)
					.await
					.unwrap();
				let root_children = get_children(&entity).await;
				let p_entity = world.entity(root_children[0]);
				let p_children = get_children(&p_entity).await;
				let expr = world.entity(p_children[0]);
				expr.get_cloned::<FileSpan>().await.unwrap()
			})
			.await
			.xpect_eq(FileSpan::new(
				"test.html",
				// span covers `name` (the expression content inside braces)
				LineCol::new(1, 4),
				LineCol::new(1, 8),
			));
	}

	#[cfg(feature = "markdown_parser")]
	#[beet_core::test]
	async fn parse_markdown_text_nodes() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				// The div contains markdown text "**bold**" which should
				// be re-parsed into <p><strong>bold</strong></p>
				HtmlParser::new()
					.with_markdown()
					.parse(entity, b"<div>**bold**</div>".to_vec(), None)
					.await
					.unwrap();

				let root_children = get_children(&entity).await;
				let div = world.entity(root_children[0]);
				let div_children = get_children(&div).await;
				// the text node should now be replaced with a subtree
				// containing a <p> with <strong>
				let first_child = world.entity(div_children[0]);
				let has_children: bool = first_child
					.with_then(|entity| entity.get::<Children>().is_some())
					.await;
				has_children
			})
			.await
			.xpect_true();
	}

	#[cfg(feature = "markdown_parser")]
	#[beet_core::test]
	async fn parse_markdown_preserves_plain_text() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				// Plain text without markdown formatting should still
				// get wrapped in a <p> element by the markdown parser
				HtmlParser::new()
					.with_markdown()
					.parse(entity, b"<div>hello world</div>".to_vec(), None)
					.await
					.unwrap();

				let root_children = get_children(&entity).await;
				let div = world.entity(root_children[0]);
				let div_children = get_children(&div).await;
				// "hello world" parsed as markdown becomes <p>hello world</p>
				// so the original text node is replaced with structure
				div_children.len()
			})
			.await
			.xpect_eq(1);
	}
}
