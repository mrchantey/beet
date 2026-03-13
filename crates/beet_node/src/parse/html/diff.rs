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

use super::combinators::HtmlParseConfig;
use super::tokens::*;
use crate::prelude::*;

use beet_core::prelude::*;
use std::borrow::Cow;

/// Returns true if an open tag named `open_tag` implicitly closes the
/// current `parent_tag`, per HTML5 optional end-tag rules.
///
/// Examples:
/// - `<body>` implicitly closes `<head>`
/// - `<li>` implicitly closes a previous `<li>`
/// - `<dt>` / `<dd>` close each other
/// - Block-level elements implicitly close `<p>`
fn implicitly_closes(parent_tag: &str, open_tag: &str) -> bool {
	match parent_tag {
		// head is implicitly closed by body or frameset
		"head" => matches!(open_tag, "body" | "frameset"),
		// li closes a previous open li
		"li" => open_tag == "li",
		// dt and dd close each other
		"dt" => matches!(open_tag, "dt" | "dd"),
		"dd" => matches!(open_tag, "dt" | "dd"),
		// p is closed by any block-level element
		"p" => {
			matches!(
				open_tag,
				"address"
					| "article" | "aside"
					| "blockquote" | "details"
					| "div" | "dl" | "fieldset"
					| "figcaption" | "figure"
					| "footer" | "form"
					| "h1" | "h2" | "h3"
					| "h4" | "h5" | "h6"
					| "header" | "hgroup"
					| "hr" | "main" | "menu"
					| "nav" | "ol" | "p"
					| "pre" | "section"
					| "summary" | "table"
					| "ul"
			)
		}
		// option / optgroup close each other
		"option" => matches!(open_tag, "option" | "optgroup"),
		"optgroup" => matches!(open_tag, "optgroup"),
		// table body / row / cell elements
		"tbody" | "thead" | "tfoot" => {
			matches!(open_tag, "tbody" | "thead" | "tfoot")
		}
		"tr" => open_tag == "tr",
		"td" | "th" => matches!(open_tag, "td" | "th"),
		_ => false,
	}
}

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
pub struct HtmlDiffConfig {
	/// Use [`Value::parse_string`] for text node content instead of [`Value::Str`].
	pub parse_text_nodes: bool,
	/// Use [`Value::parse_string`] for attribute values instead of [`Value::Str`].
	pub parse_attribute_values: bool,
	/// When `true`, decode HTML entities (eg `&amp;` → `&`) in text nodes
	/// and attribute values at parse time.
	pub unescape_html: bool,
	/// Elements that do not require a closing tag.
	pub void_elements: Vec<Cow<'static, str>>,
	/// How to handle children of void elements.
	pub void_element_children: VoidElementChildrenOpts,
	/// How to handle malformed HTML.
	pub malformed_elements: MalformedElementsOpts,
}

impl Default for HtmlDiffConfig {
	fn default() -> Self {
		Self {
			parse_text_nodes: false,
			parse_attribute_values: false,
			unescape_html: true,
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

impl HtmlDiffConfig {
	/// Returns whether the given element name is a void element.
	pub fn is_void_element(&self, name: &str) -> bool {
		let lower = name.to_ascii_lowercase();
		self.void_elements.iter().any(|el| el.as_ref() == lower)
	}

	/// Convert an attribute token value to a [`Value`] based on config.
	pub fn attribute_value(&self, raw: &str) -> Value {
		let val = if self.unescape_html {
			unescape_html_attribute(raw)
		} else {
			raw.to_string()
		};
		if self.parse_attribute_values {
			Value::parse_string(&val)
		} else {
			Value::new(val)
		}
	}

	/// Convert text content to a [`Value`] based on config.
	fn text_value(&self, raw: &str) -> Value {
		let s = if self.unescape_html {
			unescape_html_text(raw)
		} else {
			raw.to_string()
		};
		if self.parse_text_nodes {
			Value::parse_string(&s)
		} else {
			Value::new(s)
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
pub(crate) enum HtmlNode<'a> {
	/// An element with name, attributes, and children.
	Element {
		name: &'a str,
		attributes: Vec<HtmlAttribute<'a>>,
		children: Vec<HtmlNode<'a>>,
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
pub(crate) fn build_html_tree<'a>(
	tokens: &[HtmlToken<'a>],
	diff_config: &HtmlDiffConfig,
	parse_config: &HtmlParseConfig,
) -> Result<Vec<HtmlNode<'a>>> {
	let mut cursor = 0;
	build_tree_children(tokens, &mut cursor, None, diff_config, parse_config)
}

fn build_tree_children<'a>(
	tokens: &[HtmlToken<'a>],
	cursor: &mut usize,
	parent_tag: Option<&str>,
	diff_config: &HtmlDiffConfig,
	parse_config: &HtmlParseConfig,
) -> Result<Vec<HtmlNode<'a>>> {
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
				// check if this open tag implicitly closes the current parent
				if let Some(parent) = parent_tag {
					if matches!(
						diff_config.malformed_elements,
						MalformedElementsOpts::Fix
					) && implicitly_closes(parent, name)
					{
						// do not consume — let the parent caller handle it
						return Ok(children);
					}
				}

				*cursor += 1;
				let is_void = diff_config.is_void_element(name);
				let _is_raw = parse_config.is_raw_text_element(name);

				if *self_closing || is_void {
					children.push(HtmlNode::Element {
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
					children.push(HtmlNode::Element {
						name,
						attributes: attributes.clone(),
						children: element_children,
						source,
					});
				}
				continue;
			}
			HtmlToken::Text(text) => {
				children.push(HtmlNode::Text(text));
			}
			HtmlToken::Comment(text) => {
				children.push(HtmlNode::Comment(text));
			}
			HtmlToken::Doctype(text) => {
				children.push(HtmlNode::Doctype(text));
			}
			HtmlToken::Expression(expr) => {
				children.push(HtmlNode::Expression(expr));
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
	tree_nodes: &[HtmlNode<'_>],
	config: &HtmlDiffConfig,
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
	tree_node: &HtmlNode<'_>,
	config: &HtmlDiffConfig,
	span_lookup: Option<&SpanLookup>,
) -> Result {
	match tree_node {
		HtmlNode::Element {
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
		HtmlNode::Text(text) => {
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
		HtmlNode::Comment(text) => {
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
		HtmlNode::Doctype(text) => {
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
		HtmlNode::Expression(expr) => {
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
	children: &[HtmlNode<'_>],
	config: &HtmlDiffConfig,
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
	config: &HtmlDiffConfig,
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
	tree_node: &HtmlNode<'_>,
	config: &HtmlDiffConfig,
	span_lookup: Option<&SpanLookup>,
) -> Result {
	match tree_node {
		HtmlNode::Element {
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
		HtmlNode::Text(text) => {
			let value = config.text_value(text);
			let span = span_lookup.map(|lookup| lookup.span_of(text));
			let child_id = world.spawn((value, ChildOf(parent))).id();
			if let Some(span) = span {
				world.entity_mut(child_id).insert(span);
			}
		}
		HtmlNode::Comment(text) => {
			let span = span_lookup.map(|lookup| lookup.span_of(text));
			let child_id =
				world.spawn((Comment::new(*text), ChildOf(parent))).id();
			if let Some(span) = span {
				world.entity_mut(child_id).insert(span);
			}
		}
		HtmlNode::Doctype(text) => {
			let span = span_lookup.map(|lookup| lookup.span_of(text));
			let child_id =
				world.spawn((Doctype::new(*text), ChildOf(parent))).id();
			if let Some(span) = span {
				world.entity_mut(child_id).insert(span);
			}
		}
		HtmlNode::Expression(expr) => {
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
mod implicit_close_test {
	use super::implicitly_closes;

	#[test]
	fn head_closes_on_body() {
		assert!(implicitly_closes("head", "body"));
		assert!(!implicitly_closes("head", "div"));
	}

	#[test]
	fn li_closes_on_li() {
		assert!(implicitly_closes("li", "li"));
		assert!(!implicitly_closes("li", "div"));
	}

	#[test]
	fn p_closes_on_block() {
		assert!(implicitly_closes("p", "div"));
		assert!(implicitly_closes("p", "p"));
		assert!(implicitly_closes("p", "h1"));
		assert!(!implicitly_closes("p", "span"));
		assert!(!implicitly_closes("p", "em"));
	}

	#[test]
	fn dt_dd_close_each_other() {
		assert!(implicitly_closes("dt", "dd"));
		assert!(implicitly_closes("dd", "dt"));
		assert!(!implicitly_closes("dt", "li"));
	}

	#[test]
	fn unrelated_tags_do_not_close() {
		assert!(!implicitly_closes("div", "span"));
		assert!(!implicitly_closes("body", "div"));
	}
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
		let config = HtmlDiffConfig::default();
		let parse_config = HtmlParseConfig::default();
		let tree = build_html_tree(&tokens, &config, &parse_config).unwrap();
		tree.len().xpect_eq(1);
		match &tree[0] {
			HtmlNode::Element { name, children, .. } => {
				name.xpect_eq("div");
				children.len().xpect_eq(1);
				match &children[0] {
					HtmlNode::Text(text) => {
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
		let config = HtmlDiffConfig::default();
		let parse_config = HtmlParseConfig::default();
		let tree = build_html_tree(&tokens, &config, &parse_config).unwrap();
		tree.len().xpect_eq(1);
		match &tree[0] {
			HtmlNode::Element { children, .. } => {
				children.len().xpect_eq(1);
				match &children[0] {
					HtmlNode::Element { name, children, .. } => {
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
		let config = HtmlDiffConfig::default();
		let parse_config = HtmlParseConfig::default();
		let tree = build_html_tree(&tokens, &config, &parse_config).unwrap();
		// div should have two children: br (void, no children) and "after" text
		match &tree[0] {
			HtmlNode::Element { children, .. } => {
				children.len().xpect_eq(2);
				match &children[0] {
					HtmlNode::Element {
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
		let config = HtmlDiffConfig::default();
		let parse_config = HtmlParseConfig::default();
		let tree = build_html_tree(&tokens, &config, &parse_config).unwrap();
		tree.len().xpect_eq(1);
		match &tree[0] {
			HtmlNode::Element {
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
	fn build_tree_head_body_implicit_close() {
		// <head> has no close tag; <body> should implicitly close it,
		// so body ends up as a sibling of head, not nested inside it.
		let config = HtmlParseConfig::default();
		let diff_config = HtmlDiffConfig::default();
		let tokens = super::super::combinators::parse_document(
			"<html><head><meta charset=UTF-8><title>T</title><body><p>hello</p></html>",
			&config,
		)
		.unwrap();
		let tree = build_html_tree(&tokens, &diff_config, &config).unwrap();
		// tree root should be [html]
		assert_eq!(tree.len(), 1, "expected one root element");
		let HtmlNode::Element { name, children, .. } = &tree[0] else {
			panic!("expected html element");
		};
		assert_eq!(*name, "html");
		// html should have two children: head and body (not body inside head)
		let names: Vec<&str> = children
			.iter()
			.filter_map(|n| {
				if let HtmlNode::Element { name, .. } = n {
					Some(*name)
				} else {
					None
				}
			})
			.collect();
		assert_eq!(
			names,
			vec!["head", "body"],
			"head and body should be siblings under html, got {names:?}"
		);
	}

	#[test]
	fn build_tree_li_implicit_close() {
		// a second <li> should implicitly close the first
		let config = HtmlParseConfig::default();
		let diff_config = HtmlDiffConfig::default();
		let tokens = super::super::combinators::parse_document(
			"<ul><li>one<li>two</ul>",
			&config,
		)
		.unwrap();
		let tree = build_html_tree(&tokens, &diff_config, &config).unwrap();
		let HtmlNode::Element { children, .. } = &tree[0] else {
			panic!("expected ul");
		};
		let li_count = children
			.iter()
			.filter(
				|n| matches!(n, HtmlNode::Element { name, .. } if *name == "li"),
			)
			.count();
		assert_eq!(li_count, 2, "expected 2 li children, got {li_count}");
	}

	#[test]
	fn build_tree_p_implicit_close() {
		// <div> should implicitly close <p>
		let config = HtmlParseConfig::default();
		let diff_config = HtmlDiffConfig::default();
		let tokens = super::super::combinators::parse_document(
			"<div><p>text<div>inner</div></div>",
			&config,
		)
		.unwrap();
		let tree = build_html_tree(&tokens, &diff_config, &config).unwrap();
		// root div's children should be [p, div] not [p[div]]
		let HtmlNode::Element { children, .. } = &tree[0] else {
			panic!("expected outer div");
		};
		let names: Vec<&str> = children
			.iter()
			.filter_map(|n| {
				if let HtmlNode::Element { name, .. } = n {
					Some(*name)
				} else {
					None
				}
			})
			.collect();
		assert_eq!(
			names,
			vec!["p", "div"],
			"p and inner div should be siblings, got {names:?}"
		);
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
		let config = HtmlDiffConfig {
			malformed_elements: MalformedElementsOpts::Fix,
			..Default::default()
		};
		let parse_config = HtmlParseConfig::default();
		let tree = build_html_tree(&tokens, &config, &parse_config).unwrap();
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
		let config = HtmlDiffConfig {
			malformed_elements: MalformedElementsOpts::Error,
			..Default::default()
		};
		let parse_config = HtmlParseConfig::default();
		build_html_tree(&tokens, &config, &parse_config)
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
		let config = HtmlDiffConfig::default();
		let parse_config = HtmlParseConfig::default();
		let tree = build_html_tree(&tokens, &config, &parse_config).unwrap();
		match &tree[0] {
			HtmlNode::Element { children, .. } => {
				children.len().xpect_eq(2);
				match &children[0] {
					HtmlNode::Text(text) => {
						text.xpect_eq("hello ");
					}
					other => panic!("expected Text, got {other:?}"),
				}
				match &children[1] {
					HtmlNode::Element { name, .. } => {
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
		let config = HtmlDiffConfig::default();
		let parse_config = HtmlParseConfig::default();
		let tree = build_html_tree(&tokens, &config, &parse_config).unwrap();
		match &tree[0] {
			HtmlNode::Element { children, .. } => {
				children.len().xpect_eq(1);
				match &children[0] {
					HtmlNode::Expression(expr) => {
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
		let config = HtmlDiffConfig::default();
		config.is_void_element("br").xpect_true();
		config.is_void_element("BR").xpect_true();
		config.is_void_element("div").xpect_false();
		config.is_void_element("img").xpect_true();
	}

	#[test]
	fn diff_config_text_value_parsing() {
		let config = HtmlDiffConfig {
			parse_text_nodes: true,
			..Default::default()
		};
		config.text_value("42").xpect_eq(Value::Uint(42));
		config
			.text_value("hello")
			.xpect_eq(Value::Str("hello".into()));

		let config_no_parse = HtmlDiffConfig::default();
		config_no_parse
			.text_value("42")
			.xpect_eq(Value::Str("42".into()));
	}

	#[test]
	fn diff_config_attribute_value_parsing() {
		let config = HtmlDiffConfig {
			parse_attribute_values: true,
			..Default::default()
		};
		config.attribute_value("true").xpect_eq(Value::Bool(true));
		config
			.attribute_value("hello")
			.xpect_eq(Value::Str("hello".into()));

		let config_no_parse = HtmlDiffConfig::default();
		config_no_parse
			.attribute_value("true")
			.xpect_eq(Value::Str("true".into()));
	}
}
