use crate::prelude::*;
use strum_macros::AsRefStr;
use strum_macros::EnumDiscriminants;



// TODO return result, this can certainly be fallible
pub type RegisterEffect = Box<dyn FnOnce(&RsxContext)>;


#[derive(AsRefStr, EnumDiscriminants)]
pub enum RsxNode {
	/// A transparent node that simply contains children
	Fragment(Vec<RsxNode>),
	/// a rust block that returns text
	Block {
		initial: Box<RsxNode>,
		register_effect: RegisterEffect,
	},
	Doctype,
	Comment(String),
	/// may have been Text or RawText
	Text(String),
	Element(RsxElement),
	Component {
		tag: String,
		node: Box<RsxNode>,
	},
}

impl Default for RsxNode {
	fn default() -> Self { Self::Fragment(Vec::new()) }
}

impl std::fmt::Debug for RsxNode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Fragment(arg0) => {
				f.debug_tuple("Fragment").field(arg0).finish()
			}
			Self::Block { initial, .. } => f
				.debug_struct("Block")
				.field("initial", initial)
				.field("register_effect", &"..")
				.finish(),
			Self::Doctype => write!(f, "Doctype"),
			Self::Comment(arg0) => {
				f.debug_tuple("Comment").field(arg0).finish()
			}
			Self::Text(arg0) => f.debug_tuple("Text").field(arg0).finish(),
			Self::Element(arg0) => {
				f.debug_tuple("Element").field(arg0).finish()
			}
			Self::Component { tag, node } => f
				.debug_struct("Component")
				.field("tag", tag)
				.field("node", node)
				.finish(),
		}
	}
}

impl RsxNode {
	pub fn discriminant(&self) -> RsxNodeDiscriminants { self.into() }
	pub fn is_element(&self) -> bool { matches!(self, RsxNode::Element(_)) }

	pub fn children(&self) -> &[RsxNode] {
		match self {
			RsxNode::Fragment(rsx_nodes) => rsx_nodes,
			RsxNode::Block { initial, .. } => initial.children(),
			RsxNode::Element(RsxElement { children, .. }) => &children,
			_ => &[],
		}
	}
	pub fn children_mut(&mut self) -> &mut [RsxNode] {
		match self {
			RsxNode::Fragment(rsx_nodes) => rsx_nodes,
			RsxNode::Block { initial, .. } => initial.children_mut(),
			RsxNode::Element(RsxElement { children, .. }) => children,
			_ => &mut [],
		}
	}


	/// A method used by macros to insert nodes into a slot
	/// # Panics
	/// If the slot is not found
	pub fn with_slots(mut self, name: &str, nodes: Vec<RsxNode>) -> Self {
		match self.try_insert_slots(name, nodes) {
			Some(_) => {
				panic!("slot not found: {name}");
			}
			None => return self,
		}
	}

	// try to insert nodes into a slot, returning any nodes that were not inserted
	// If the slot is not a direct child, recursively search children
	pub fn try_insert_slots(
		&mut self,
		name: &str,
		mut nodes: Vec<Self>,
	) -> Option<Vec<Self>> {
		match self {
			RsxNode::Fragment(fragment) => {
				for node in fragment.iter_mut() {
					match node.try_insert_slots(name, nodes) {
						Some(returned_nodes) => nodes = returned_nodes,
						None => return None,
					}
				}
				Some(nodes)
			}
			RsxNode::Element(element) => {
				if element.tag == "slot" {
					let slot_name = element
						.attributes
						.iter()
						.find_map(|a| match a {
							RsxAttribute::KeyValue { key, value } => {
								if key == "name" {
									Some(value.as_str())
								} else {
									None
								}
							}
							// even block values are not allowed because we need slot names at macro time
							_ => None,
						})
						// unnamed slots are called 'default'
						.unwrap_or("default");
					if slot_name == name {
						element.children.extend(nodes);
						return None;
					}
				}
				// if we didnt find the slot, recursively search children
				for child in &mut element.children {
					match child.try_insert_slots(name, nodes) {
						Some(returned_nodes) => nodes = returned_nodes,
						None => return None,
					}
				}
				Some(nodes)
			}
			_ => Some(nodes),
		}
	}

	/// takes all the register_effect functions
	pub fn register_effects(&mut self) {
		fn call_effect(cx: &RsxContext, register_effect: &mut RegisterEffect) {
			let func = std::mem::replace(register_effect, Box::new(|_| {}));
			func(cx);
		}
		RsxContext::visit_mut(self, |cx, node| match node {
			RsxNode::Block {
				register_effect, ..
			} => {
				call_effect(cx, register_effect);
			}
			RsxNode::Element(e) => {
				for a in &mut e.attributes {
					match a {
						RsxAttribute::Block {
							register_effect, ..
						} => call_effect(cx, register_effect),
						RsxAttribute::BlockValue {
							register_effect, ..
						} => call_effect(cx, register_effect),
						_ => {}
					}
				}
			}
			_ => {}
		});
	}

}
