//! Entity tree diffing logic for the markdown parser.
//!
//! Diffs an [`HtmlNode`] tree (built by [`tree_builder`](super::tree_builder)
//! from `pulldown-cmark` events and BSX fragment tokens) against any existing
//! entity hierarchy to minimize mutations on reparse. Embedded markup that needs
//! the full BSX grammar (an uppercase component/template tag, a `bx:` directive,
//! or a `{..}` spread) is built through [`BsxTemplate`] so BSX is the single
//! markup authority; plain HTML elements diff in place here.
//!
//! When a [`SpanLookup`] is provided, every spawned or diffed entity
//! receives a [`FileSpan`] component covering its source text:
//! - Element entities: span of the opening tag (`<tag ...>`)
//! - Attribute entities: span of the `key="value"` text, when locatable
//! - Text nodes: span of the text content
//! - Comment nodes: span of the comment text
//! - Doctype nodes: span of the doctype text
//! - Expression nodes: span of the `{expr}` text

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
	///
	/// HTML tag names are case-insensitive (`<BR>` is `<br>`), but BSX uses case to
	/// distinguish a component/template from an element. A PascalCase tag (eg the
	/// `<Link>` widget) is therefore never the HTML void element it lowercases to
	/// (`link`): it resolves through the BSX path and keeps its slot children.
	pub fn is_void_element(&self, name: &str) -> bool {
		if is_component_tag(name) {
			return false;
		}
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
	///
	/// Attributes share the BSX [`AttrValue`] grammar, so a `bx:` directive or a
	/// spread resolves natively rather than as a stringly-typed pair.
	Element {
		name: &'a str,
		attributes: Vec<BsxAttribute>,
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

impl<'a> HtmlNode<'a> {
	/// Whether this node is whitespace-only text, ie carries no visible content.
	pub(crate) fn is_blank_text(&self) -> bool {
		matches!(self, HtmlNode::Text(text) if text.trim().is_empty())
	}

	/// Whether this node is a block-level element (see [`is_block_element`]).
	/// Text and non-visual nodes (comments, doctype, expressions) are not block
	/// boxes, so they join the surrounding inline run.
	pub(crate) fn is_block_level(&self) -> bool {
		matches!(self, HtmlNode::Element { name, .. } if is_block_element(name))
	}

	/// Whether this element must be built through the BSX resolver rather than
	/// diffed as a plain HTML element: an uppercase component/template tag, or any
	/// attribute carrying a `bx:` directive, a value-grammar expression, or a
	/// spread. Plain HTML (the markdown-heavy common case) returns `false`.
	fn needs_bsx(&self) -> bool {
		let HtmlNode::Element {
			name, attributes, ..
		} = self
		else {
			return false;
		};
		is_bsx_tag(name)
			|| attributes.iter().any(|attr| {
				attr.key.starts_with("bx:")
					|| matches!(
						attr.value,
						AttrValue::Expr(_) | AttrValue::Spread(_)
					)
			})
	}
}

/// Whether a tag resolves by name (component/template) rather than as an HTML
/// element: a capitalized tag, or a `path::to::X` whose last segment is
/// capitalized. Mirrors the core BSX `is_uppercase_tag`.
fn is_bsx_tag(tag: &str) -> bool {
	tag.rsplit("::")
		.next()
		.unwrap_or(tag)
		.starts_with(|ch: char| ch.is_uppercase())
}

/// Whether a tag is unambiguously a BSX component/template rather than an HTML
/// element written in uppercase. A component is PascalCase (an uppercase first
/// char *and* a lowercase letter, eg `Link`) or carries a `::` path; an all-caps
/// tag (`BR`, `LINK`) stays an HTML element, since HTML tag names are
/// case-insensitive. Used to keep an uppercase component tag from colliding with
/// a void HTML element it lowercases to.
fn is_component_tag(tag: &str) -> bool {
	let last = tag.rsplit("::").next().unwrap_or(tag);
	tag.contains("::")
		|| (last.starts_with(|ch: char| ch.is_uppercase())
			&& last.chars().any(|ch| ch.is_lowercase()))
}

/// Build an [`HtmlNode::Element`] subtree through [`BsxTemplate`], so an embedded
/// uppercase component/template, `bx:` directive, or spread resolves through the
/// one BSX resolver. Replaces the former post-parse `resolve_mdx_templates` pass:
/// resolution now happens per-tag, inline, while the tree is diffed.
fn build_via_bsx(world: &mut World, entity: Entity, node: &HtmlNode<'_>) {
	let Some(element) = html_node_to_bsx(node) else {
		return;
	};
	// despawn any existing children and strip stale markers before rebuilding.
	let children: Vec<Entity> = world
		.entity(entity)
		.get::<Children>()
		.map(|kids| kids.iter().collect())
		.unwrap_or_default();
	for child in children {
		world.entity_mut(child).despawn();
	}
	world.entity_mut(entity).remove::<Element>();
	let registry = world
		.get_resource::<BsxTemplateRegistry>()
		.cloned()
		.unwrap_or_default();
	let template = BsxTemplate::new(vec![element], registry);
	// a build failure rides `TemplateError`/`LoadTemplate` on the entity.
	let _ = world.entity_mut(entity).insert_template(template);
}

/// Convert an [`HtmlNode`] tree into a [`BsxNode`], for the BSX build path.
fn html_node_to_bsx(node: &HtmlNode<'_>) -> Option<BsxNode> {
	match node {
		HtmlNode::Element {
			name,
			attributes,
			children,
			..
		} => Some(BsxNode::Element(BsxElement {
			tag: name.to_string(),
			attributes: attributes.clone(),
			children: children.iter().filter_map(html_node_to_bsx).collect(),
			self_closing: children.is_empty(),
		})),
		HtmlNode::Text(text) => {
			Some(BsxNode::Text(unescape_html_text(text)))
		}
		HtmlNode::Comment(text) => Some(BsxNode::Comment(text.to_string())),
		HtmlNode::Doctype(text) => Some(BsxNode::Doctype(text.to_string())),
		// a markdown `{expr}` inside a BSX subtree: parse it as a value expression.
		HtmlNode::Expression(expr) => parse_text_block_expr(expr),
	}
}

/// Parse a markdown expression string into a BSX text-position value expression.
fn parse_text_block_expr(expr: &str) -> Option<BsxNode> {
	parse_value_expr_str(expr).ok().map(BsxNode::Expr)
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
			// an embedded BSX element (uppercase tag, `bx:`, or spread) resolves
			// through the one BSX resolver rather than diffing as plain HTML.
			if tree_node.needs_bsx() {
				let span = span_lookup.map(|lookup| lookup.span_of(source));
				build_via_bsx(world, entity, tree_node);
				if let Some(span) = span {
					world.entity_mut(entity).insert(span);
				}
				return Ok(());
			}
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
				diff_attributes(world, entity, attributes, config)?;

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
	attributes: &[BsxAttribute],
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
	diff_attributes(world, entity, attributes, config)?;

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

/// Diff the attributes of a plain HTML element against its entity.
///
/// Only [`AttrValue::Str`] and [`AttrValue::Flag`] reach here: a `bx:` directive,
/// a value-grammar expression, or a spread routes the whole element through the
/// BSX resolver instead ([`build_via_bsx`]), so this stays the simple HTML case.
fn diff_attributes(
	world: &mut World,
	entity: Entity,
	attributes: &[BsxAttribute],
	config: &HtmlDiffConfig,
) -> Result {
	// pre-convert to owned `(key, value)` pairs; a `Flag` has no value.
	let parse_values = config.parse_attribute_values;
	let attrs_owned: Vec<(String, Value)> = attributes
		.iter()
		.map(|attr| {
			let value = match &attr.value {
				AttrValue::Str(string) if parse_values => {
					Value::parse_string(string)
				}
				AttrValue::Str(string) => Value::str(string),
				_ => Value::Null,
			};
			(attr.key.clone(), value)
		})
		.collect();

	// collect existing attribute entities as (entity, key, value).
	let existing: Vec<(Entity, String, Value)> = world
		.entity(entity)
		.get::<Attributes>()
		.map(|attrs| attrs.iter().collect::<Vec<_>>())
		.unwrap_or_default()
		.into_iter()
		.filter_map(|attr_entity| {
			let entity_ref = world.get_entity(attr_entity).ok()?;
			let key = entity_ref.get::<Attribute>()?.to_string();
			let value = entity_ref.get::<Value>().cloned().unwrap_or_default();
			Some((attr_entity, key, value))
		})
		.collect();

	let mut matched = vec![false; existing.len()];
	for (key, new_value) in &attrs_owned {
		match existing.iter().position(|(_, ex_key, _)| ex_key == key) {
			Some(idx) => {
				matched[idx] = true;
				let (attr_entity, _, existing_val) = &existing[idx];
				if existing_val != new_value {
					world.entity_mut(*attr_entity).insert(new_value.clone());
				}
			}
			None => {
				world.spawn((
					Attribute::new(key),
					new_value.clone(),
					AttributeOf::new(entity),
				));
			}
		}
	}

	// despawn unmatched existing attributes.
	for (idx, was_matched) in matched.iter().enumerate() {
		if !was_matched {
			world.entity_mut(existing[idx].0).despawn();
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
			let child_id = world.spawn(ChildOf(parent)).id();
			if let Some(span) = span {
				world.entity_mut(child_id).insert(span);
			}
			// an embedded BSX element resolves through the BSX resolver.
			if tree_node.needs_bsx() {
				build_via_bsx(world, child_id, tree_node);
			} else {
				world.entity_mut(child_id).insert(Element::new(*name));
				diff_attributes(world, child_id, attributes, config)?;
				for child_node in children {
					spawn_node(world, child_id, child_node, config, span_lookup)?;
				}
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
mod test {
	use super::*;

	#[beet_core::test]
	fn diff_config_is_void() {
		let config = HtmlDiffConfig::default();
		config.is_void_element("br").xpect_true();
		config.is_void_element("BR").xpect_true();
		config.is_void_element("div").xpect_false();
		config.is_void_element("img").xpect_true();
	}

	#[beet_core::test]
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

	#[beet_core::test]
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

	/// A capitalized BSX component tag (eg the `<Link>` widget) is never the HTML
	/// void element it lowercases to (`<link>`), so its slot children survive.
	/// Regression for the landing-page CTA rendering as an empty `<a></a>` with its
	/// label leaking out as a sibling, because `<Link>` matched the void `link`.
	#[beet_core::test]
	fn bsx_component_tag_is_not_void() {
		let config = HtmlDiffConfig::default();
		// the HTML void element stays void, in any case (HTML is case-insensitive).
		config.is_void_element("link").xpect_true();
		config.is_void_element("LINK").xpect_true();
		// the PascalCase component tag is not — it keeps its slot children.
		config.is_void_element("Link").xpect_false();
	}

	/// The component's children are nested, not flattened into siblings: a
	/// `<Link>text</Link>` parses to a `Link` element holding the text, ready for
	/// the BSX build path to route into its default slot.
	#[cfg(feature = "markdown_parser")]
	#[beet_core::test]
	fn markdown_component_keeps_children() {
		use crate::parse::markdown::tree_builder::build_markdown_tree;
		let tree = build_markdown_tree(
			"<Link href=\"/x\">hello</Link>",
			crate::prelude::MarkdownParseConfig::default_cmark_options(),
			&crate::prelude::HtmlParseConfig::default(),
			&HtmlDiffConfig::default(),
			None,
		)
		.unwrap();
		let describe = |node: &HtmlNode| match node {
			HtmlNode::Element { name, children, .. } => {
				format!("{name}/{}", children.len())
			}
			_ => "?".to_string(),
		};
		// one node: the Link element holding its single text child (not Link/0
		// beside a stray text sibling).
		tree.nodes.iter().map(describe).collect::<Vec<_>>().xpect_eq(vec![
			"Link/1".to_string(),
		]);
	}
}
