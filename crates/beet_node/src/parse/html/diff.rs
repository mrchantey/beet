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
//! - Comment nodes: span of the comment text
//! - Doctype nodes: span of the doctype text
//! - Expression nodes: span of the `{expr}` text

use super::combinators::ParseConfig;
use super::tokens::*;
use crate::prelude::*;
use beet_core::prelude::*;
use std::borrow::Cow;

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
pub(crate) enum TreeNode<'a> {
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
pub(crate) fn build_tree<'a>(
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
fn collect_children(world: &World, entity: Entity) -> Vec<Entity> {
	world
		.entity(entity)
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
pub(crate) fn diff_children(
	world: &mut World,
	entity: Entity,
	tree_nodes: &[TreeNode<'_>],
	config: &DiffConfig,
	span_lookup: Option<&SpanLookup>,
) -> Result {
	let existing_children = collect_children(world, entity);
	let existing_count = existing_children.len();
	let new_count = tree_nodes.len();

	// diff each tree node against existing children
	for (idx, tree_node) in tree_nodes.iter().enumerate() {
		if idx < existing_count {
			let child_entity = existing_children[idx];
			diff_node(world, child_entity, tree_node, config, span_lookup)?;
		} else {
			spawn_node(world, entity, tree_node, config, span_lookup)?;
		}
	}

	// despawn excess children
	if existing_count > new_count {
		let excess: Vec<Entity> = existing_children[new_count..].to_vec();
		for child in excess {
			world.entity_mut(child).despawn();
		}
	}

	Ok(())
}

/// Diff a single tree node against an existing entity, updating in place
/// if the type matches, or replacing it if the type differs.
///
/// When `span_lookup` is provided, a [`FileSpan`] is inserted on the entity.
fn diff_node(
	world: &mut World,
	entity: Entity,
	tree_node: &TreeNode<'_>,
	config: &DiffConfig,
	span_lookup: Option<&SpanLookup>,
) -> Result {
	match tree_node {
		TreeNode::Element {
			name,
			attributes,
			children,
			source,
		} => {
			let span = span_lookup.map(|lookup| lookup.span_of(source));
			let has_matching_element =
				world.entity(entity).get::<Element>().is_some();

			if has_matching_element {
				// update element name and span in place
				let mut entity_mut = world.entity_mut(entity);
				entity_mut.remove::<Element>();
				entity_mut.insert(Element::new(*name));
				if let Some(span) = span {
					entity_mut.insert(span);
				}
				drop(entity_mut);

				// diff attributes
				diff_attributes(
					world,
					entity,
					attributes,
					config,
					span_lookup,
				)?;

				// diff children recursively
				diff_children(world, entity, children, config, span_lookup)?;
			} else {
				// type mismatch: replace entity contents
				replace_with_element(
					world,
					entity,
					name,
					attributes,
					children,
					config,
					span_lookup,
					span,
				)?;
			}
		}
		TreeNode::Text(text) => {
			let value = config.text_value(text);
			let span = span_lookup.map(|lookup| lookup.span_of(text));
			let mut entity_mut = world.entity_mut(entity);
			// remove element-related components if present
			entity_mut.remove::<Element>();
			entity_mut.remove::<Comment>();
			entity_mut.remove::<Doctype>();
			entity_mut.remove::<Expression>();
			entity_mut.set_if_ne_or_insert(value);
			if let Some(span) = span {
				entity_mut.set_if_ne_or_insert(span);
			}
		}
		TreeNode::Comment(text) => {
			let span = span_lookup.map(|lookup| lookup.span_of(text));
			let mut entity_mut = world.entity_mut(entity);
			entity_mut.remove::<Element>();
			entity_mut.remove::<Doctype>();
			entity_mut.remove::<Expression>();
			entity_mut.remove::<Value>();
			// Comment is immutable, remove and re-insert
			entity_mut.remove::<Comment>();
			entity_mut.insert(Comment::new(*text));
			if let Some(span) = span {
				entity_mut.set_if_ne_or_insert(span);
			}
		}
		TreeNode::Doctype(text) => {
			let span = span_lookup.map(|lookup| lookup.span_of(text));
			let mut entity_mut = world.entity_mut(entity);
			entity_mut.remove::<Element>();
			entity_mut.remove::<Comment>();
			entity_mut.remove::<Expression>();
			entity_mut.remove::<Value>();
			// Doctype is immutable, remove and re-insert
			entity_mut.remove::<Doctype>();
			entity_mut.insert(Doctype::new(*text));
			if let Some(span) = span {
				entity_mut.set_if_ne_or_insert(span);
			}
		}
		TreeNode::Expression(expr) => {
			let expression = Expression(expr.to_string());
			let span = span_lookup.map(|lookup| lookup.span_of(expr));
			let mut entity_mut = world.entity_mut(entity);
			entity_mut.remove::<Element>();
			entity_mut.remove::<Comment>();
			entity_mut.remove::<Doctype>();
			// Expression is immutable, remove and re-insert
			entity_mut.remove::<Expression>();
			entity_mut.insert(expression);
			if let Some(span) = span {
				entity_mut.set_if_ne_or_insert(span);
			}
		}
	}
	Ok(())
}

/// Replace an entity's contents with a new element, clearing old components.
fn replace_with_element(
	world: &mut World,
	entity: Entity,
	name: &str,
	attributes: &[HtmlAttribute<'_>],
	children: &[TreeNode<'_>],
	config: &DiffConfig,
	span_lookup: Option<&SpanLookup>,
	span: Option<FileSpan>,
) -> Result {
	// clear and set element
	let mut entity_mut = world.entity_mut(entity);
	entity_mut.remove::<Comment>();
	entity_mut.remove::<Doctype>();
	entity_mut.remove::<Expression>();
	entity_mut.remove::<Value>();
	entity_mut.remove::<Element>();
	entity_mut.insert(Element::new(name));
	if let Some(span) = span {
		entity_mut.set_if_ne_or_insert(span);
	}
	drop(entity_mut);

	// diff attributes
	diff_attributes(world, entity, attributes, config, span_lookup)?;

	// despawn all existing children
	let child_ids = collect_children(world, entity);
	for child in child_ids {
		if world.get_entity(child).is_ok() {
			world.entity_mut(child).despawn();
		}
	}

	// spawn new children
	for child_node in children {
		spawn_node(world, entity, child_node, config, span_lookup)?;
	}

	Ok(())
}

/// Diff attributes on an entity against a list of parsed attributes.
///
/// When `span_lookup` is provided, each attribute entity receives a
/// [`FileSpan`] covering its source text (key, `=`, and value).
fn diff_attributes(
	world: &mut World,
	entity: Entity,
	attributes: &[HtmlAttribute<'_>],
	config: &DiffConfig,
	span_lookup: Option<&SpanLookup>,
) -> Result {
	// pre-convert to owned data including optional spans
	let attrs_owned: Vec<(String, Option<String>, bool, Option<FileSpan>)> =
		attributes
			.iter()
			.map(|attr| {
				let span = span_lookup.and_then(|lookup| {
					if !attr.key.is_empty() {
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

	// collect existing attribute entities
	let existing_attr_entities: Vec<Entity> = world
		.entity(entity)
		.get::<Attributes>()
		.map(|attrs| {
			let mut ids = Vec::new();
			for attr_entity in attrs.iter() {
				ids.push(attr_entity);
			}
			ids
		})
		.unwrap_or_default();

	// build a list of existing attributes: (entity, key, value)
	let existing: Vec<(Entity, String, Value)> = existing_attr_entities
		.iter()
		.filter_map(|&attr_entity| {
			let entity_ref = world.get_entity(attr_entity).ok()?;
			let key = entity_ref.get::<Attribute>()?.to_string();
			let value = entity_ref.get::<Value>().cloned().unwrap_or_default();
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
			let found = existing
				.iter()
				.position(|(_, existing_key, _)| existing_key == key);

			if let Some(idx) = found {
				matched[idx] = true;
				let (attr_entity, _, ref existing_val) = existing[idx];
				if *existing_val != expr_value {
					world.entity_mut(attr_entity).insert(expr_value);
				}
				// ensure it has the Expression component
				// Expression is immutable, remove then insert
				let mut attr_mut = world.entity_mut(attr_entity);
				attr_mut.remove::<Expression>();
				attr_mut.insert(Expression(value.clone().unwrap_or_default()));
				if let Some(span) = span {
					attr_mut.insert(span.clone());
				}
			} else {
				// spawn new attribute entity
				let bundle = (
					Attribute::new(key),
					expr_value,
					Expression(value.clone().unwrap_or_default()),
					AttributeOf::new(entity),
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
			let found = existing
				.iter()
				.position(|(_, existing_key, _)| *existing_key == *key);

			if let Some(idx) = found {
				matched[idx] = true;
				let (attr_entity, _, ref existing_val) = existing[idx];
				if *existing_val != new_value {
					world.entity_mut(attr_entity).insert(new_value);
				}
				if let Some(span) = span {
					world.entity_mut(attr_entity).insert(span.clone());
				}
			} else {
				// spawn new attribute entity
				let attr_id = world
					.spawn((
						Attribute::new(key),
						new_value,
						AttributeOf::new(entity),
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

	Ok(())
}

/// Spawn a new child entity from a tree node.
///
/// When `span_lookup` is provided, the new entity receives a [`FileSpan`].
pub(crate) fn spawn_node(
	world: &mut World,
	parent: Entity,
	tree_node: &TreeNode<'_>,
	config: &DiffConfig,
	span_lookup: Option<&SpanLookup>,
) -> Result {
	match tree_node {
		TreeNode::Element {
			name,
			attributes,
			children,
			source,
		} => {
			let span = span_lookup.map(|lookup| lookup.span_of(source));
			let child_id =
				world.spawn((Element::new(*name), ChildOf(parent))).id();
			if let Some(span) = span {
				world.entity_mut(child_id).insert(span);
			}
			diff_attributes(world, child_id, attributes, config, span_lookup)?;
			for child_node in children {
				spawn_node(world, child_id, child_node, config, span_lookup)?;
			}
		}
		TreeNode::Text(text) => {
			let value = config.text_value(text);
			let span = span_lookup.map(|lookup| lookup.span_of(text));
			let child_id = world.spawn((value, ChildOf(parent))).id();
			if let Some(span) = span {
				world.entity_mut(child_id).insert(span);
			}
		}
		TreeNode::Comment(text) => {
			let span = span_lookup.map(|lookup| lookup.span_of(text));
			let child_id =
				world.spawn((Comment::new(*text), ChildOf(parent))).id();
			if let Some(span) = span {
				world.entity_mut(child_id).insert(span);
			}
		}
		TreeNode::Doctype(text) => {
			let span = span_lookup.map(|lookup| lookup.span_of(text));
			let child_id =
				world.spawn((Doctype::new(*text), ChildOf(parent))).id();
			if let Some(span) = span {
				world.entity_mut(child_id).insert(span);
			}
		}
		TreeNode::Expression(expr) => {
			let span = span_lookup.map(|lookup| lookup.span_of(expr));
			let child_id = world
				.spawn((Expression(expr.to_string()), ChildOf(parent)))
				.id();
			if let Some(span) = span {
				world.entity_mut(child_id).insert(span);
			}
		}
	}
	Ok(())
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
