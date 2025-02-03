use crate::prelude::*;


pub struct RsxElement {
	/// ie `div, span, input`
	pub tag: String,
	/// ie `class="my-class"`
	pub attributes: Vec<RsxAttribute>,
	/// ie `<div>childtext<childel/>{childblock}</div>`
	pub children: Vec<RsxNode>,
	/// ie `<input/>`
	pub self_closing: bool,
}


impl RsxElement {
	pub fn new(tag: String, self_closing: bool) -> Self {
		Self {
			tag,
			self_closing,
			attributes: Vec::new(),
			children: Vec::new(),
		}
	}



	/// non-recursive check for blocks in children
	pub fn contains_blocks(&self) -> bool {
		self.children
			.iter()
			.any(|c| matches!(c, RsxNode::Block { .. }))
	}

	/// Whether any children or attributes are blocks,
	/// used to determine whether the node requires an id
	pub fn contains_rust(&self) -> bool {
		self.contains_blocks()
			|| self.attributes.iter().any(|a| {
				matches!(
					a,
					RsxAttribute::Block { .. }
						| RsxAttribute::BlockValue { .. }
				)
			})
	}
}

impl std::fmt::Debug for RsxElement {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("RsxElement")
			.field("tag", &self.tag)
			.field("attributes", &self.attributes)
			.field("children", &self.children)
			.field("self_closing", &self.self_closing)
			.finish()
	}
}

// #[derive(Debug, Clone, PartialEq)]
// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum RsxAttribute {
	Key {
		key: String,
	},
	KeyValue {
		key: String,
		value: String,
	},
	BlockValue {
		key: String,
		initial: String,
		register_effect: RegisterEffect,
	},
	// kind of like a fragment, but for attributes
	Block {
		initial: Vec<RsxAttribute>,
		register_effect: RegisterEffect,
	},
}

impl RsxAttribute {}


impl std::fmt::Debug for RsxAttribute {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Key { key } => {
				f.debug_struct("Key").field("key", key).finish()
			}
			Self::KeyValue { key, value } => f
				.debug_struct("KeyValue")
				.field("key", key)
				.field("value", value)
				.finish(),
			Self::BlockValue { key, initial, .. } => f
				.debug_struct("BlockValue")
				.field("key", key)
				.field("initial", initial)
				.field("register_effect", &"..")
				.finish(),
			Self::Block { initial, .. } => f
				.debug_struct("Block")
				.field("initial", initial)
				.field("register_effect", &"..")
				.finish(),
		}
	}
}
