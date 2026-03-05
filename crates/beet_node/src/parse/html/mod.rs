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
use std::borrow::Cow;

/// A configurable HTML parser that implements [`NodeParser`].
///
/// Parses HTML into a tree of ECS entities, diffing against any existing
/// hierarchy to avoid unnecessary mutations. Supports expressions,
/// void elements, raw text elements, and configurable error handling.
#[derive(Debug, Clone)]
pub struct HtmlParser {
	/// Use [`Value::parse_string`] for text node content instead of [`Value::Str`].
	pub parse_text_nodes: bool,
	/// Use [`Value::parse_string`] for attribute values instead of [`Value::Str`].
	pub parse_attribute_values: bool,
	/// Enable `{expr}` expression parsing in content and attributes.
	pub parse_expressions: bool,
	/// Enable `{{expr}}` double-escaped expressions in raw text elements.
	pub parse_raw_text_expressions: bool,
	/// Elements that do not require a closing tag.
	pub void_elements: Vec<Cow<'static, str>>,
	/// How to handle children of void elements.
	pub void_element_children: VoidElementChildrenOpts,
	/// How to handle malformed HTML.
	pub malformed_elements: MalformedElementsOpts,
	/// Element names whose content is raw text, ie `<script>`, `<style>`.
	pub raw_text_elements: Vec<Cow<'static, str>>,
	/// Element names whose content is raw character data, ie `<textarea>`, `<title>`.
	pub raw_character_data_elements: Vec<Cow<'static, str>>,
}

impl Default for HtmlParser {
	fn default() -> Self {
		Self {
			parse_text_nodes: false,
			parse_attribute_values: false,
			parse_expressions: false,
			parse_raw_text_expressions: false,
			void_elements: vec![
				"area".into(),
				"base".into(),
				"br".into(),
				"col".into(),
				"embed".into(),
				"hr".into(),
				"img".into(),
				"input".into(),
				"link".into(),
				"meta".into(),
				"param".into(),
				"source".into(),
				"track".into(),
				"wbr".into(),
			],
			void_element_children: VoidElementChildrenOpts::default(),
			malformed_elements: MalformedElementsOpts::default(),
			raw_text_elements: vec!["script".into(), "style".into()],
			raw_character_data_elements: vec![
				"textarea".into(),
				"title".into(),
			],
		}
	}
}

impl HtmlParser {
	/// Create a new parser with default HTML5 settings.
	pub fn new() -> Self { Self::default() }

	/// Create a parser with expression support enabled.
	pub fn with_expressions() -> Self {
		Self {
			parse_expressions: true,
			parse_raw_text_expressions: true,
			..Default::default()
		}
	}

	/// Build the internal [`ParseConfig`] from this parser's settings.
	fn parse_config(&self) -> ParseConfig {
		ParseConfig {
			parse_expressions: self.parse_expressions,
			parse_raw_text_expressions: self.parse_raw_text_expressions,
			raw_text_elements: self
				.raw_text_elements
				.iter()
				.map(|el| el.to_string())
				.collect(),
			raw_character_data_elements: self
				.raw_character_data_elements
				.iter()
				.map(|el| el.to_string())
				.collect(),
		}
	}

	/// Build the internal [`DiffConfig`] from this parser's settings.
	fn diff_config(&self) -> DiffConfig {
		DiffConfig {
			parse_text_nodes: self.parse_text_nodes,
			parse_attribute_values: self.parse_attribute_values,
			void_elements: self.void_elements.clone(),
			void_element_children: self.void_element_children.clone(),
			malformed_elements: self.malformed_elements.clone(),
		}
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

			let parse_config = self.parse_config();
			let diff_config = self.diff_config();

			// tokenize
			let tokens = combinators::parse_document(text, &parse_config)?;

			// build tree from flat tokens
			let tree = build_tree(&tokens, &diff_config, &parse_config)?;

			// set up span tracker if path was provided
			let span_tracker = path.as_ref().map(|path| {
				let mut tracker = SpanTracker::new(path.clone());
				tracker.extend(text);
				tracker
			});

			// diff tree against entity, note the root is not a node so is not diffed
			diff_children(&entity, &tree, &diff_config, span_tracker.as_ref())
				.await?;

			// insert file span on the root entity if path provided
			if let Some(tracker) = span_tracker {
				let span = tracker.into_full_span();
				entity
					.with_then(move |mut entity| {
						entity.set_if_ne_or_insert(span);
					})
					.await;
			}

			Ok(())
		}
	}

	fn parse_stream(
		&mut self,
		_entity: AsyncEntity,
		stream: impl 'static
		+ Unpin
		+ bevy::tasks::futures_lite::Stream<
			Item = Result<impl AsRef<[u8]>>,
		>,
		_path: Option<WsPathBuf>,
	) -> impl Future<Output = Result> {
		async move {
			let mut stream = stream_ext::bytes_to_text(stream);
			while let Some(_result) = stream.next().await {
				todo!("parse chunk, shared impl with parse()");
			}
			Ok(())
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
				let mut parser = HtmlParser::new();
				parser.parse_text_nodes = true;
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
}
