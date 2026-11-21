//! [`ReadElement`], an HTML element an Office reader assembles before it
//! spawns the tree.
use super::office_style::*;
use beet_core::prelude::*;

/// An element a reader assembles in memory, then spawns whole: its tag,
/// attributes, children and the look the file formats on it directly.
#[derive(Debug, Clone)]
pub(crate) struct ReadElement {
	/// The tag, ie `td`.
	pub tag: SmolStr,
	/// The attributes in order, a boolean attribute's value [`Value::Null`].
	pub attributes: Vec<(SmolStr, Value)>,
	/// The children in order.
	pub children: Vec<ReadNode>,
	/// What the file formats directly on it.
	pub look: DirectLook,
}

/// A child of a [`ReadElement`].
#[derive(Debug, Clone)]
pub(crate) enum ReadNode {
	/// An element.
	Element(ReadElement),
	/// Text.
	Text(String),
}

impl ReadNode {
	/// `nodes` with each run of adjacent texts joined into one, an empty
	/// text dropped.
	pub fn merge_text(nodes: Vec<ReadNode>) -> Vec<ReadNode> {
		let mut out = Vec::<ReadNode>::with_capacity(nodes.len());
		for node in nodes {
			match (out.last_mut(), node) {
				(_, ReadNode::Text(text)) if text.is_empty() => {}
				(Some(ReadNode::Text(last)), ReadNode::Text(text)) => {
					last.push_str(&text)
				}
				(_, node) => out.push(node),
			}
		}
		out
	}
}

impl ReadElement {
	/// An element `tag` with nothing in it.
	pub fn new(tag: impl Into<SmolStr>) -> Self {
		Self {
			tag: tag.into(),
			attributes: Vec::new(),
			children: Vec::new(),
			look: DirectLook::default(),
		}
	}

	/// Adds the attribute `key` holding `value`.
	pub fn with_attribute(
		mut self,
		key: impl Into<SmolStr>,
		value: impl Into<Value>,
	) -> Self {
		self.attributes.push((key.into(), value.into()));
		self
	}

	/// Adds the boolean attribute `key`, ie `hidden`.
	pub fn with_flag(mut self, key: impl Into<SmolStr>) -> Self {
		self.attributes.push((key.into(), Value::Null));
		self
	}

	/// Adds `child`.
	pub fn with_child(mut self, child: ReadElement) -> Self {
		self.children.push(ReadNode::Element(child));
		self
	}

	/// Adds `text`, unless it is empty.
	pub fn with_text(mut self, text: impl Into<String>) -> Self {
		let text = text.into();
		if !text.is_empty() {
			self.children.push(ReadNode::Text(text));
		}
		self
	}

	/// Sets the look the file formats directly on it.
	pub fn with_look(mut self, look: DirectLook) -> Self {
		self.look = look;
		self
	}

	/// Whether `other` is this element's shell: its tag, attributes and
	/// look, its children aside, so a reader merges runs of equal wrappers.
	pub fn same_shell(&self, other: &Self) -> bool {
		self.tag == other.tag
			&& self.attributes == other.attributes
			&& self.look == other.look
	}

	/// Spawns this element and its subtree as the last child of `parent`,
	/// each look registered by `options` against `theme`.
	pub fn spawn(
		self,
		world: &mut World,
		parent: Entity,
		options: &OfficeReadOptions,
		theme: &OfficeTheme,
	) -> Result<Entity> {
		let entity =
			world.spawn((Element::new(self.tag), ChildOf(parent))).id();
		for (key, value) in self.attributes {
			world.spawn((AttributeOf::new(entity), Attribute::new(key), value));
		}
		if !self.look.is_empty() {
			self.look
				.apply(&mut world.entity_mut(entity), options, theme)?;
		}
		for child in self.children {
			match child {
				ReadNode::Element(element) => {
					element.spawn(world, entity, options, theme)?;
				}
				ReadNode::Text(text) => {
					world.spawn((Value::Str(text.into()), ChildOf(entity)));
				}
			}
		}
		entity.xok()
	}
}
