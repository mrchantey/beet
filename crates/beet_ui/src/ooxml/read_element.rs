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

/// One piece of inline content inside the elements wrapping it, outermost
/// first, before equal wrappers are merged: a reader pushes one per run and
/// [`assemble`](Self::assemble)s a paragraph's.
#[derive(Debug, Clone)]
pub(crate) struct ReadSegment {
	/// The elements wrapping it, outermost first.
	pub wrappers: Vec<ReadElement>,
	/// The content itself.
	pub node: ReadNode,
}

/// A list item as a reader reads it, before its list is assembled: its
/// depth from 0 and whether its list counts.
#[derive(Debug, Clone)]
pub(crate) struct ReadListItem {
	/// Its depth, 0 for a top level item.
	pub level: u8,
	/// Whether its list counts, an `ol`, else a `ul`.
	pub ordered: bool,
	/// The `li`.
	pub element: ReadElement,
}

impl ReadListItem {
	/// One list at `depth` of `items`, a deeper item nested in the item
	/// before it, the list's kind its first item's.
	pub fn list(items: &[ReadListItem], depth: u8) -> ReadElement {
		let tag = if items[0].ordered { "ol" } else { "ul" };
		let mut list = ReadElement::new(tag);
		let mut index = 0;
		while index < items.len() {
			if items[index].level <= depth {
				list = list.with_child(items[index].element.clone());
				index += 1;
				continue;
			}
			let end = (index..items.len())
				.find(|end| items[*end].level <= depth)
				.unwrap_or(items.len());
			let nested = Self::list(&items[index..end], depth + 1);
			match list
				.children
				.iter_mut()
				.rev()
				.find_map(|child| match child {
					ReadNode::Element(element) => Some(element),
					ReadNode::Text(_) => None,
				}) {
				Some(last) => last.children.push(ReadNode::Element(nested)),
				None => {
					list = list
						.with_child(ReadElement::new("li").with_child(nested))
				}
			}
			index = end;
		}
		list
	}
}

impl ReadSegment {
	/// The content of `segments`, each run of equal wrappers one element.
	pub fn assemble(segments: &[ReadSegment]) -> Vec<ReadNode> {
		Self::assemble_from(segments, 0)
	}

	/// The content of `segments` from their wrappers at `depth` in.
	fn assemble_from(segments: &[ReadSegment], depth: usize) -> Vec<ReadNode> {
		let mut out = Vec::new();
		let mut index = 0;
		while index < segments.len() {
			let segment = &segments[index];
			let Some(wrapper) = segment.wrappers.get(depth) else {
				out.push(segment.node.clone());
				index += 1;
				continue;
			};
			let end = (index..segments.len())
				.find(|end| {
					segments[*end]
						.wrappers
						.get(depth)
						.is_none_or(|other| !wrapper.same_shell(other))
				})
				.unwrap_or(segments.len());
			let mut element = wrapper.clone();
			element.children =
				Self::assemble_from(&segments[index..end], depth + 1);
			out.push(ReadNode::Element(element));
			index = end;
		}
		ReadNode::merge_text(out)
	}
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

	/// This cell holding `blocks`: a lone plain paragraph's content held
	/// directly, its alignment, background and attributes the cell's, else
	/// the blocks.
	pub fn with_cell_content(mut self, blocks: Vec<ReadElement>) -> Self {
		match <[ReadElement; 1]>::try_from(blocks) {
			Ok([paragraph]) if paragraph.tag == "p" => {
				let look = &paragraph.look;
				self.look.text_align = look.text_align;
				self.look.background = self.look.background.or(look.background);
				self.attributes.extend(paragraph.attributes);
				self.children = paragraph.children;
				self
			}
			Ok(blocks) => blocks.into_iter().fold(self, Self::with_child),
			Err(blocks) => blocks.into_iter().fold(self, Self::with_child),
		}
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
