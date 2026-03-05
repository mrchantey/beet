//! Entity tree diffing logic for the HTML parser.
//!
//! Converts a flat stream of [`HtmlToken`] into a tree of ECS entities,
//! diffing against any existing entity hierarchy to minimize mutations.
//!
//! When a [`SpanLookup`] is provided, every spawned or diffed entity
//! receives a [`FileSpan`] component covering its source text:
//! - Element entities: span of the opening tag (`<tag ...>`)
//! - Attribute entities: span of the `key="value"` or `{expr}` text
//! - Text nodes: span of the text content
//! - Comment / doctype nodes: span of the comment / doctype text
//! - Expression nodes: span of the `{expr}` text

use super::combinators::ParseConfig;
use super::tokens::*;
use crate::prelude::*;
use beet_core::prelude::*;
use std::borrow::Cow;
use std::pin::Pin;

/// Controls how children of void elements are handled.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum VoidElementChildrenOpts {
	/// Allow children of void elements.
	#[default]
	Preserve,
	/// Children of void elements are moved to subsequent siblings (browser behavior).
	Pop,
	/// Error on children of void elements.
	Error,
}

/// Controls how malformed HTML is handled.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum MalformedElementsOpts {
	/// Attempt to parse missing closing tags etc anyway (browser behavior).
	#[default]
	Fix,
	/// Error on missing closing tags etc.
	Error,
}

/// Configuration for the HTML differ, controlling how tokens are applied to entities.
#[derive(Debug, Clone)]
pub struct DiffConfig {
	/// Use [`Value::parse_string`] for text node content instead of [`Value::Str`].
	pub parse_text_nodes: bool,
	/// Use [`Value::parse_string`] for attribute values instead of [`Value::Str`].
	pub parse_attribute_values: bool,
	/// Elements that do not require a closing tag.
	pub void_elements: Vec<Cow<'static, str>>,
	/// How to handle children of void elements.
	pub void_element_children: VoidElementChildrenOpts,
	/// How to handle malformed HTML.
	pub malformed_elements: MalformedElementsOpts,
}

impl Default for DiffConfig {
	fn default() -> Self {
		Self {
			parse_text_nodes: false,
			parse_attribute_values: false,
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
		}
	}
}

impl DiffConfig {
	/// Returns whether the given element name is a void element.
	pub fn is_void_element(&self, name: &str) -> bool {
		let lower = name.to_ascii_lowercase();
		self.void_elements.iter().any(|el| el.as_ref() == lower)
	}

	/// Convert an attribute token value to a [`Value`] based on config.
	pub fn attribute_value(&self, raw: &str) -> Value {
		if self.parse_attribute_values {
			Value::parse_string(raw)
		} else {
			Value::new(raw)
		}
	}

	/// Convert text content to a [`Value`] based on config.
	fn text_value(&self, raw: &str) -> Value {
		if self.parse_text_nodes {
			Value::parse_string(raw)
		} else {
			Value::new(raw)
		}
	}
}

/// A tree node built from the flat token stream, used as an intermediate
/// representation before diffing against the entity tree.
///
/// Borrows string slices from the input for zero-copy parsing.
/// Each variant carries enough source information for [`SpanLookup`]
/// to produce a [`FileSpan`].
#[derive(Debug, Clone, PartialEq)]
pub(super) enum TreeNode<'a> {
	/// An element with name, attributes, and children.
	Element {
		name: &'a str,
		attributes: Vec<HtmlAttribute<'a>>,
		children: Vec<TreeNode<'a>>,
		/// The full opening tag source text, ie `<div class="foo">`.
		source: &'a str,
	},
	/// A text node.
	Text(&'a str),
	/// An HTML comment.
	Comment(&'a str),
	/// A doctype declaration.
	Doctype(&'a str),
	/// An expression `{expr}`.
	Expression(&'a str),
}

/// Build a tree of [`TreeNode`] from a flat list of [`HtmlToken`].
///
/// This handles nesting by tracking open/close tags, void elements,
/// and self-closing tags. Malformed HTML is handled per the config.
pub(super) fn build_tree<'a>(
	tokens: &[HtmlToken<'a>],
	diff_config: &DiffConfig,
	parse_config: &ParseConfig,
) -> Result<Vec<TreeNode<'a>>> {
	let mut cursor = 0;
	build_tree_children(tokens, &mut cursor, None, diff_config, parse_config)
}

fn build_tree_children<'a>(
	tokens: &[HtmlToken<'a>],
	cursor: &mut usize,
	parent_tag: Option<&str>,
	diff_config: &DiffConfig,
	parse_config: &ParseConfig,
) -> Result<Vec<TreeNode<'a>>> {
	let mut children = Vec::new();

	while *cursor < tokens.len() {
		let token = &tokens[*cursor];
		match token {
			HtmlToken::CloseTag(name) => {
				// does this close tag match our parent?
				if let Some(parent) = parent_tag {
					if name.eq_ignore_ascii_case(parent) {
						// consume the close tag and return
						*cursor += 1;
						return Ok(children);
					}
					// mismatched close tag
					match diff_config.malformed_elements {
						MalformedElementsOpts::Error => {
							bevybail!(
								"unexpected closing tag </{name}>, expected </{parent}>"
							);
						}
						MalformedElementsOpts::Fix => {
							// implicitly close current element, do not consume
							// this close tag (it belongs to an ancestor)
							return Ok(children);
						}
					}
				} else {
					// close tag at root level
					match diff_config.malformed_elements {
						MalformedElementsOpts::Error => {
							bevybail!(
								"unexpected closing tag </{name}> at root level"
							);
						}
						MalformedElementsOpts::Fix => {
							// skip it
							*cursor += 1;
							continue;
						}
					}
				}
			}
			HtmlToken::OpenTag {
				name,
				attributes,
				self_closing,
				source,
			} => {
				*cursor += 1;
				let is_void = diff_config.is_void_element(name);
				let _is_raw = parse_config.is_raw_text_element(name);

				if *self_closing || is_void {
					children.push(TreeNode::Element {
						name,
						attributes: attributes.clone(),
						children: vec![],
						source,
					});
				} else {
					// recurse to collect children until the matching close tag
					let element_children = build_tree_children(
						tokens,
						cursor,
						Some(name),
						diff_config,
						parse_config,
					)?;
					children.push(TreeNode::Element {
						name,
						attributes: attributes.clone(),
						children: element_children,
						source,
					});
				}
				continue;
			}
			HtmlToken::Text(text) => {
				children.push(TreeNode::Text(text));
			}
			HtmlToken::Comment(text) => {
				children.push(TreeNode::Comment(text));
			}
			HtmlToken::Doctype(text) => {
				children.push(TreeNode::Doctype(text));
			}
			HtmlToken::Expression(expr) => {
				children.push(TreeNode::Expression(expr));
			}
		}
		*cursor += 1;
	}

	// if we had a parent tag and reached EOF without closing it
	if let Some(parent) = parent_tag {
		match diff_config.malformed_elements {
			MalformedElementsOpts::Error => {
				bevybail!("unclosed element <{parent}>");
			}
			MalformedElementsOpts::Fix => {
				// implicitly closed
			}
		}
	}

	Ok(children)
}

/// Collect the entity ids of the direct children of `entity`.
fn collect_children(entity: &EntityWorldMut) -> Vec<Entity> {
	entity
		.get::<Children>()
		.map(|children| {
			let mut result = Vec::new();
			for &child in children {
				result.push(child);
			}
			result
		})
		.unwrap_or_default()
}

/// Apply a list of [`TreeNode`] as children of the given entity,
/// diffing against existing children to minimize ECS mutations.
pub(super) async fn diff_children(
	entity: &AsyncEntity,
	tree_nodes: &[TreeNode<'_>],
	config: &DiffConfig,
	span_lookup: Option<&SpanLookup>,
) -> Result {
	let existing_children: Vec<Entity> =
		entity.with_then(|entity| collect_children(&entity)).await;

	let existing_count = existing_children.len();
	let new_count = tree_nodes.len();

	// process each tree node against existing children
	for (idx, tree_node) in tree_nodes.iter().enumerate() {
		if idx < existing_count {
			// diff against existing child
			let child_entity = existing_children[idx];
			let child = entity.world().entity(child_entity);
			diff_node(&child, tree_node, config, span_lookup).await?;
		} else {
			// spawn new child
			spawn_node(entity, tree_node, config, span_lookup).await?;
		}
	}

	// despawn excess children
	if existing_count > new_count {
		let world = entity.world().clone();
		let excess: Vec<Entity> = existing_children[new_count..].to_vec();
		world
			.with_then(move |world: &mut World| {
				for child in excess {
					if world.get_entity(child).is_ok() {
						let entity_mut: EntityWorldMut =
							world.entity_mut(child);
						entity_mut.despawn();
					}
				}
			})
			.await;
	}

	Ok(())
}

/// Diff a single tree node against an existing entity, updating in place
/// if the type matches, or replacing it if the type differs.
///
/// When `span_lookup` is provided, a [`FileSpan`] is inserted on the entity.
fn diff_node<'a, 'b>(
	entity: &'a AsyncEntity,
	tree_node: &'b TreeNode<'_>,
	config: &'a DiffConfig,
	span_lookup: Option<&'a SpanLookup>,
) -> Pin<Box<dyn Future<Output = Result> + 'a>>
where
	'b: 'a,
{
	Box::pin(async move {
		match tree_node {
			TreeNode::Element {
				name,
				attributes,
				children,
				source,
			} => {
				let el_name = name.to_string();
				let span = span_lookup.map(|lookup| lookup.span_of(source));
				let attrs_owned: Vec<HtmlAttribute> = attributes
					.iter()
					.map(|attr| HtmlAttribute {
						key: attr.key,
						value: attr.value,
						expression: attr.expression,
					})
					.collect();
				let has_matching_element = entity
					.with_then(move |entity| entity.get::<Element>().is_some())
					.await;

				if has_matching_element {
					// update element name and span in place
					let el_name_clone = el_name.clone();
					entity
						.with_then(move |mut entity| {
							// Element is immutable, remove and re-insert
							entity.remove::<Element>();
							entity.insert(Element::new(el_name_clone));
							if let Some(span) = span {
								entity.set_if_ne_or_insert(span);
							}
						})
						.await;

					// diff attributes
					diff_attributes(entity, &attrs_owned, config, span_lookup)
						.await?;

					// diff children recursively
					diff_children(entity, children, config, span_lookup)
						.await?;
				} else {
					// type mismatch: replace entity contents
					replace_with_element(
						entity,
						&el_name,
						&attrs_owned,
						children,
						config,
						span_lookup,
						span,
					)
					.await?;
				}
			}
			TreeNode::Text(text) => {
				let value = config.text_value(text);
				let span = span_lookup.map(|lookup| lookup.span_of(text));
				entity
					.with_then(move |mut entity| {
						// remove element-related components if present
						entity.remove::<Element>();
						entity.remove::<Comment>();
						entity.remove::<Expression>();
						entity.set_if_ne_or_insert(value);
						if let Some(span) = span {
							entity.set_if_ne_or_insert(span);
						}
					})
					.await;
			}
			TreeNode::Comment(text) => {
				let value = Value::Str(text.to_string());
				let span = span_lookup.map(|lookup| lookup.span_of(text));
				entity
					.with_then(move |mut entity| {
						entity.remove::<Element>();
						entity.remove::<Expression>();
						// Comment is immutable, remove and re-insert
						entity.remove::<Comment>();
						entity.insert(Comment);
						entity.set_if_ne_or_insert(value);
						if let Some(span) = span {
							entity.set_if_ne_or_insert(span);
						}
					})
					.await;
			}
			TreeNode::Doctype(text) => {
				// doctypes are stored as comments with a special value
				let value = Value::Str(text.to_string());
				let span = span_lookup.map(|lookup| lookup.span_of(text));
				entity
					.with_then(move |mut entity| {
						entity.remove::<Element>();
						entity.remove::<Expression>();
						entity.remove::<Comment>();
						entity.insert(Comment);
						entity.set_if_ne_or_insert(value);
						if let Some(span) = span {
							entity.set_if_ne_or_insert(span);
						}
					})
					.await;
			}
			TreeNode::Expression(expr) => {
				let expression = Expression(expr.to_string());
				let span = span_lookup.map(|lookup| lookup.span_of(expr));
				entity
					.with_then(move |mut entity| {
						entity.remove::<Element>();
						entity.remove::<Comment>();
						// Expression is immutable, remove and re-insert
						entity.remove::<Expression>();
						entity.insert(expression);
						if let Some(span) = span {
							entity.set_if_ne_or_insert(span);
						}
					})
					.await;
			}
		}
		Ok(())
	})
}

/// Replace an entity's contents with a new element, clearing old components.
async fn replace_with_element(
	entity: &AsyncEntity,
	name: &str,
	attributes: &[HtmlAttribute<'_>],
	children: &[TreeNode<'_>],
	config: &DiffConfig,
	span_lookup: Option<&SpanLookup>,
	span: Option<FileSpan>,
) -> Result {
	let el_name = name.to_string();
	// clear and set element
	entity
		.with_then(move |mut entity| {
			entity.remove::<Comment>();
			entity.remove::<Expression>();
			entity.remove::<Value>();
			// Element is immutable, remove before inserting
			entity.remove::<Element>();
			entity.insert(Element::new(el_name));
			if let Some(span) = span {
				entity.set_if_ne_or_insert(span);
			}
		})
		.await;

	// set attributes
	diff_attributes(entity, attributes, config, span_lookup).await?;

	// despawn all existing children and spawn new ones
	let world = entity.world().clone();
	let parent_id = entity.id();

	world
		.with_then(move |world: &mut World| {
			let child_ids: Vec<Entity> = {
				let entity_ref = world.entity(parent_id);
				entity_ref
					.get::<Children>()
					.map(|children| {
						let mut ids = Vec::new();
						for &child in children {
							ids.push(child);
						}
						ids
					})
					.unwrap_or_default()
			};
			for child in child_ids {
				let entity_mut: EntityWorldMut = world.entity_mut(child);
				entity_mut.despawn();
			}
		})
		.await;

	// spawn new children
	for child_node in children {
		spawn_node(entity, child_node, config, span_lookup).await?;
	}

	Ok(())
}

/// Diff attributes on an entity against a list of parsed attributes.
///
/// This batches all attribute work into a single `with_then` call
/// to minimize world locking overhead.
///
/// When `span_lookup` is provided, each attribute entity receives a
/// [`FileSpan`] covering its source text (key, `=`, and value).
async fn diff_attributes(
	entity: &AsyncEntity,
	attributes: &[HtmlAttribute<'_>],
	config: &DiffConfig,
	span_lookup: Option<&SpanLookup>,
) -> Result {
	// pre-convert to owned data for the closure, including optional spans
	let attrs_owned: Vec<(String, Option<String>, bool, Option<FileSpan>)> =
		attributes
			.iter()
			.map(|attr| {
				// compute span from the key slice if available and non-empty
				let span = span_lookup.and_then(|lookup| {
					if !attr.key.is_empty() {
						// for keyed attributes, span covers from key start
						// through value end (if present)
						if let Some(val) = attr.value {
							// span from key to end of value
							let key_offset = lookup.slice_offset(attr.key);
							let val_offset = lookup.slice_offset(val);
							let end_offset = val_offset + val.len();
							Some(FileSpan::new(
								lookup.path().as_ref(),
								lookup.line_col(key_offset),
								lookup.line_col(end_offset),
							))
						} else {
							Some(lookup.span_of(attr.key))
						}
					} else if let Some(val) = attr.value {
						// keyless expression: span covers the expression content
						Some(lookup.span_of(val))
					} else {
						None
					}
				});
				(
					attr.effective_key().to_string(),
					attr.value.map(|val| val.to_string()),
					attr.expression,
					span,
				)
			})
			.collect();
	let parse_values = config.parse_attribute_values;

	entity
		.with_then(move |mut entity| {
			// collect existing attribute entities
			let existing_attr_entities: Vec<Entity> = entity
				.get::<Attributes>()
				.map(|attrs| {
					let mut ids = Vec::new();
					for attr_entity in attrs.iter() {
						ids.push(attr_entity);
					}
					ids
				})
				.unwrap_or_default();

			let entity_id = entity.id();

			entity.world_scope(move |world: &mut World| {
				// build a list of existing attributes: (entity, key, value)
				let existing: Vec<(Entity, String, Value)> =
					existing_attr_entities
						.iter()
						.filter_map(|&attr_entity| {
							let entity_ref: EntityRef =
								world.get_entity(attr_entity).ok()?;
							let key: String =
								entity_ref.get::<Attribute>()?.to_string();
							let value: Value = entity_ref
								.get::<Value>()
								.cloned()
								.unwrap_or_default();
							Some((attr_entity, key, value))
						})
						.collect();

				// process new attributes
				let mut matched = vec![false; existing.len()];

				for (key, value, is_expression, span) in &attrs_owned {
					if *is_expression {
						// expression attributes
						let expr_value: Value = value
							.as_ref()
							.map(|val| Value::Str(val.clone()))
							.unwrap_or(Value::Null);

						// try to find matching existing attribute
						let found =
							existing.iter().position(|(_, existing_key, _)| {
								existing_key == key
							});

						if let Some(idx) = found {
							matched[idx] = true;
							let (attr_entity, _, ref existing_val) =
								existing[idx];
							if *existing_val != expr_value {
								let mut attr_mut: EntityWorldMut =
									world.entity_mut(attr_entity);
								attr_mut.insert(expr_value);
							}
							// also ensure it has the Expression component
							// Expression is immutable, remove then insert
							let mut attr_mut: EntityWorldMut =
								world.entity_mut(attr_entity);
							attr_mut.remove::<Expression>();
							attr_mut.insert(Expression(
								value.clone().unwrap_or_default(),
							));
							if let Some(span) = span {
								attr_mut.insert(span.clone());
							}
						} else {
							// spawn new attribute entity
							let bundle = (
								Attribute::new(key),
								expr_value,
								Expression(value.clone().unwrap_or_default()),
								AttributeOf::new(entity_id),
							);
							let attr_id = world.spawn(bundle).id();
							if let Some(span) = span {
								world.entity_mut(attr_id).insert(span.clone());
							}
						}
					} else {
						let new_value: Value = value
							.as_ref()
							.map(|val| {
								if parse_values {
									Value::parse_string(val)
								} else {
									Value::Str(val.clone())
								}
							})
							.unwrap_or(Value::Null);

						// find matching existing attribute
						let found =
							existing.iter().position(|(_, existing_key, _)| {
								*existing_key == *key
							});

						if let Some(idx) = found {
							matched[idx] = true;
							let (attr_entity, _, ref existing_val) =
								existing[idx];
							if *existing_val != new_value {
								let mut attr_mut: EntityWorldMut =
									world.entity_mut(attr_entity);
								attr_mut.insert(new_value);
							}
							if let Some(span) = span {
								let mut attr_mut: EntityWorldMut =
									world.entity_mut(attr_entity);
								attr_mut.insert(span.clone());
							}
						} else {
							// spawn new attribute entity
							let attr_id = world
								.spawn((
									Attribute::new(key),
									new_value,
									AttributeOf::new(entity_id),
								))
								.id();
							if let Some(span) = span {
								world.entity_mut(attr_id).insert(span.clone());
							}
						}
					}
				}

				// despawn unmatched existing attributes
				for (idx, was_matched) in matched.iter().enumerate() {
					if !was_matched {
						let (attr_entity, _, _) = &existing[idx];
						world.entity_mut(*attr_entity).despawn();
					}
				}
			});
		})
		.await;

	Ok(())
}

/// Spawn a new child entity from a tree node.
///
/// When `span_lookup` is provided, the new entity receives a [`FileSpan`].
fn spawn_node<'a>(
	parent: &'a AsyncEntity,
	tree_node: &'a TreeNode<'_>,
	config: &'a DiffConfig,
	span_lookup: Option<&'a SpanLookup>,
) -> Pin<Box<dyn Future<Output = Result> + 'a>> {
	Box::pin(async move {
		match tree_node {
			TreeNode::Element {
				name,
				attributes,
				children,
				source,
			} => {
				let span = span_lookup.map(|lookup| lookup.span_of(source));
				let child_entity =
					parent.spawn_child(Element::new(*name)).await;
				let child = parent.world().entity(child_entity);
				if let Some(span) = span {
					child
						.with_then(move |mut entity| {
							entity.insert(span);
						})
						.await;
				}
				diff_attributes(&child, attributes, config, span_lookup)
					.await?;
				for child_node in children {
					spawn_node(&child, child_node, config, span_lookup).await?;
				}
			}
			TreeNode::Text(text) => {
				let value = config.text_value(text);
				let span = span_lookup.map(|lookup| lookup.span_of(text));
				let child_entity = parent.spawn_child(value).await;
				if let Some(span) = span {
					let child = parent.world().entity(child_entity);
					child
						.with_then(move |mut entity| {
							entity.insert(span);
						})
						.await;
				}
			}
			TreeNode::Comment(text) => {
				let value = Value::Str(text.to_string());
				let span = span_lookup.map(|lookup| lookup.span_of(text));
				let child_entity = parent.spawn_child((Comment, value)).await;
				if let Some(span) = span {
					let child = parent.world().entity(child_entity);
					child
						.with_then(move |mut entity| {
							entity.insert(span);
						})
						.await;
				}
			}
			TreeNode::Doctype(text) => {
				let value = Value::Str(text.to_string());
				let span = span_lookup.map(|lookup| lookup.span_of(text));
				let child_entity = parent.spawn_child((Comment, value)).await;
				if let Some(span) = span {
					let child = parent.world().entity(child_entity);
					child
						.with_then(move |mut entity| {
							entity.insert(span);
						})
						.await;
				}
			}
			TreeNode::Expression(expr) => {
				let span = span_lookup.map(|lookup| lookup.span_of(expr));
				let child_entity =
					parent.spawn_child(Expression(expr.to_string())).await;
				if let Some(span) = span {
					let child = parent.world().entity(child_entity);
					child
						.with_then(move |mut entity| {
							entity.insert(span);
						})
						.await;
				}
			}
		}
		Ok(())
	})
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn build_tree_simple_element() {
		let tokens = vec![
			HtmlToken::OpenTag {
				name: "div",
				attributes: vec![],
				self_closing: false,
				source: "<div>",
			},
			HtmlToken::Text("hello"),
			HtmlToken::CloseTag("div"),
		];
		let config = DiffConfig::default();
		let parse_config = ParseConfig::default();
		let tree = build_tree(&tokens, &config, &parse_config).unwrap();
		tree.len().xpect_eq(1);
		match &tree[0] {
			TreeNode::Element { name, children, .. } => {
				name.xpect_eq("div");
				children.len().xpect_eq(1);
				match &children[0] {
					TreeNode::Text(text) => {
						text.xpect_eq("hello");
					}
					other => panic!("expected Text, got {other:?}"),
				}
			}
			other => panic!("expected Element, got {other:?}"),
		}
	}

	#[test]
	fn build_tree_nested() {
		let tokens = vec![
			HtmlToken::OpenTag {
				name: "div",
				attributes: vec![],
				self_closing: false,
				source: "<div>",
			},
			HtmlToken::OpenTag {
				name: "span",
				attributes: vec![],
				self_closing: false,
				source: "<span>",
			},
			HtmlToken::Text("inner"),
			HtmlToken::CloseTag("span"),
			HtmlToken::CloseTag("div"),
		];
		let config = DiffConfig::default();
		let parse_config = ParseConfig::default();
		let tree = build_tree(&tokens, &config, &parse_config).unwrap();
		tree.len().xpect_eq(1);
		match &tree[0] {
			TreeNode::Element { children, .. } => {
				children.len().xpect_eq(1);
				match &children[0] {
					TreeNode::Element { name, children, .. } => {
						name.xpect_eq("span");
						children.len().xpect_eq(1);
					}
					other => panic!("expected Element, got {other:?}"),
				}
			}
			other => panic!("expected Element, got {other:?}"),
		}
	}

	#[test]
	fn build_tree_void_element() {
		let tokens = vec![
			HtmlToken::OpenTag {
				name: "div",
				attributes: vec![],
				self_closing: false,
				source: "<div>",
			},
			HtmlToken::OpenTag {
				name: "br",
				attributes: vec![],
				self_closing: false,
				source: "<br>",
			},
			HtmlToken::Text("after"),
			HtmlToken::CloseTag("div"),
		];
		let config = DiffConfig::default();
		let parse_config = ParseConfig::default();
		let tree = build_tree(&tokens, &config, &parse_config).unwrap();
		// div should have two children: br (void, no children) and "after" text
		match &tree[0] {
			TreeNode::Element { children, .. } => {
				children.len().xpect_eq(2);
				match &children[0] {
					TreeNode::Element {
						name,
						children: br_children,
						..
					} => {
						name.xpect_eq("br");
						br_children.len().xpect_eq(0);
					}
					other => panic!("expected br Element, got {other:?}"),
				}
			}
			other => panic!("expected Element, got {other:?}"),
		}
	}

	#[test]
	fn build_tree_self_closing() {
		let tokens = vec![HtmlToken::OpenTag {
			name: "img",
			attributes: vec![HtmlAttribute::new("src", "foo.png")],
			self_closing: true,
			source: "<img src=\"foo.png\" />",
		}];
		let config = DiffConfig::default();
		let parse_config = ParseConfig::default();
		let tree = build_tree(&tokens, &config, &parse_config).unwrap();
		tree.len().xpect_eq(1);
		match &tree[0] {
			TreeNode::Element {
				name,
				attributes,
				children,
				..
			} => {
				name.xpect_eq("img");
				attributes.len().xpect_eq(1);
				children.len().xpect_eq(0);
			}
			other => panic!("expected Element, got {other:?}"),
		}
	}

	#[test]
	fn build_tree_malformed_fix() {
		// missing close tag for <span>, should be implicitly closed
		let tokens = vec![
			HtmlToken::OpenTag {
				name: "div",
				attributes: vec![],
				self_closing: false,
				source: "<div>",
			},
			HtmlToken::OpenTag {
				name: "span",
				attributes: vec![],
				self_closing: false,
				source: "<span>",
			},
			HtmlToken::Text("hello"),
			HtmlToken::CloseTag("div"),
		];
		let config = DiffConfig {
			malformed_elements: MalformedElementsOpts::Fix,
			..Default::default()
		};
		let parse_config = ParseConfig::default();
		let tree = build_tree(&tokens, &config, &parse_config).unwrap();
		tree.len().xpect_eq(1);
	}

	#[test]
	fn build_tree_malformed_error() {
		let tokens = vec![
			HtmlToken::OpenTag {
				name: "div",
				attributes: vec![],
				self_closing: false,
				source: "<div>",
			},
			HtmlToken::Text("hello"),
			// missing close tag for div
		];
		let config = DiffConfig {
			malformed_elements: MalformedElementsOpts::Error,
			..Default::default()
		};
		let parse_config = ParseConfig::default();
		build_tree(&tokens, &config, &parse_config)
			.unwrap_err()
			.to_string()
			.xpect_contains("unclosed");
	}

	#[test]
	fn build_tree_mixed_content() {
		let tokens = vec![
			HtmlToken::OpenTag {
				name: "p",
				attributes: vec![],
				self_closing: false,
				source: "<p>",
			},
			HtmlToken::Text("hello "),
			HtmlToken::OpenTag {
				name: "em",
				attributes: vec![],
				self_closing: false,
				source: "<em>",
			},
			HtmlToken::Text("world"),
			HtmlToken::CloseTag("em"),
			HtmlToken::CloseTag("p"),
		];
		let config = DiffConfig::default();
		let parse_config = ParseConfig::default();
		let tree = build_tree(&tokens, &config, &parse_config).unwrap();
		match &tree[0] {
			TreeNode::Element { children, .. } => {
				children.len().xpect_eq(2);
				match &children[0] {
					TreeNode::Text(text) => {
						text.xpect_eq("hello ");
					}
					other => panic!("expected Text, got {other:?}"),
				}
				match &children[1] {
					TreeNode::Element { name, .. } => {
						name.xpect_eq("em");
					}
					other => panic!("expected Element, got {other:?}"),
				}
			}
			other => panic!("expected Element, got {other:?}"),
		}
	}

	#[test]
	fn build_tree_expression() {
		let tokens = vec![
			HtmlToken::OpenTag {
				name: "div",
				attributes: vec![],
				self_closing: false,
				source: "<div>",
			},
			HtmlToken::Expression("foo"),
			HtmlToken::CloseTag("div"),
		];
		let config = DiffConfig::default();
		let parse_config = ParseConfig::default();
		let tree = build_tree(&tokens, &config, &parse_config).unwrap();
		match &tree[0] {
			TreeNode::Element { children, .. } => {
				children.len().xpect_eq(1);
				match &children[0] {
					TreeNode::Expression(expr) => {
						expr.xpect_eq("foo");
					}
					other => panic!("expected Expression, got {other:?}"),
				}
			}
			other => panic!("expected Element, got {other:?}"),
		}
	}

	#[test]
	fn diff_config_is_void() {
		let config = DiffConfig::default();
		config.is_void_element("br").xpect_true();
		config.is_void_element("BR").xpect_true();
		config.is_void_element("div").xpect_false();
		config.is_void_element("img").xpect_true();
	}

	#[test]
	fn diff_config_text_value_parsing() {
		let config = DiffConfig {
			parse_text_nodes: true,
			..Default::default()
		};
		config.text_value("42").xpect_eq(Value::Uint(42));
		config
			.text_value("hello")
			.xpect_eq(Value::Str("hello".into()));

		let config_no_parse = DiffConfig::default();
		config_no_parse
			.text_value("42")
			.xpect_eq(Value::Str("42".into()));
	}

	#[test]
	fn diff_config_attribute_value_parsing() {
		let config = DiffConfig {
			parse_attribute_values: true,
			..Default::default()
		};
		config.attribute_value("true").xpect_eq(Value::Bool(true));
		config
			.attribute_value("hello")
			.xpect_eq(Value::Str("hello".into()));

		let config_no_parse = DiffConfig::default();
		config_no_parse
			.attribute_value("true")
			.xpect_eq(Value::Str("true".into()));
	}
}
