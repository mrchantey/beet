//! Streaming HTML parser that diffs parsed content against an existing entity tree.
//!
//! This module implements [`NodeParser`] for HTML, using `winnow` parser
//! combinators to tokenize the input and a depth-first differ to apply
//! minimal ECS mutations.
//!
//! Enable with the `html_parser` feature flag.

mod combinators;
mod diff;
mod tokens;

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
}

impl Default for HtmlParser {
	fn default() -> Self {
		Self {
			parse_config: ParseConfig::default(),
			diff_config: DiffConfig::default(),
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
		}
	}

	/// Shared parsing logic: tokenize, build tree, diff against entity.
	///
	/// This is the core implementation used by both [`NodeParser::parse`]
	/// and the streaming path. It operates synchronously on a complete
	/// text buffer with full world access via `AsyncEntity`.
	async fn parse_text(
		&self,
		entity: &AsyncEntity,
		text: &str,
		path: Option<&WsPathBuf>,
	) -> Result {
		let parse_config = &self.parse_config;
		let diff_config = &self.diff_config;

		// tokenize
		let tokens = combinators::parse_document(text, parse_config)?;

		// build tree from flat tokens
		let tree = build_tree(&tokens, diff_config, parse_config)?;

		// build span lookup if path was provided, enabling per-entity FileSpan
		let span_lookup = path.map(|path| SpanLookup::new(text, path.clone()));

		// diff tree against entity, note the root is not a node so is not diffed
		diff_children(entity, &tree, diff_config, span_lookup.as_ref()).await?;

		// insert file span on the root entity if path provided
		if let Some(ref lookup) = span_lookup {
			let span = lookup.full_span();
			entity
				.with_then(move |mut entity| {
					entity.set_if_ne_or_insert(span);
				})
				.await;
		}

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
				let has_comment: bool = child
					.with_then(|entity| entity.get::<Comment>().is_some())
					.await;

				has_comment
			})
			.await
			.xpect_true();
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
}
