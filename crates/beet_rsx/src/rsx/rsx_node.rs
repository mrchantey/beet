use crate::prelude::*;
use strum_macros::AsRefStr;
use strum_macros::EnumDiscriminants;


#[derive(Debug, AsRefStr, EnumDiscriminants)]
pub enum RsxNode {
	/// A transparent node that simply contains children
	/// This may be deprecated in the future if no patterns
	/// require it. The RstmlToRsx could support it
	Fragment(Vec<RsxNode>),
	Component {
		tag: String,
		/// used to resolve with templates
		tracker: Option<RustyTracker>,
		node: Box<RsxNode>,
	},
	/// a rust block that returns text
	Block(RsxBlock),
	/// a html element
	Element(RsxElement),
	/// a html text node
	Text(String),
	/// a html comment node
	Comment(String),
	/// a html doctype node
	Doctype,
}

impl Default for RsxNode {
	fn default() -> Self { Self::Fragment(Vec::new()) }
}

impl RsxNode {
	pub fn discriminant(&self) -> RsxNodeDiscriminants { self.into() }
	pub fn is_element(&self) -> bool { matches!(self, RsxNode::Element(_)) }


	/// chidren of root, fragment or element.
	/// Blocks and components have no children
	pub fn children(&self) -> &[RsxNode] {
		match self {
			RsxNode::Fragment(rsx_nodes) => rsx_nodes,
			RsxNode::Component { .. } => &[],
			RsxNode::Block(RsxBlock { initial, .. }) => initial.children(),
			RsxNode::Element(RsxElement { children, .. }) => &children,
			RsxNode::Text(_) => &[],
			RsxNode::Comment(_) => &[],
			RsxNode::Doctype => &[],
		}
	}
	/// chidren of root, fragment or element.
	/// Blocks and components have no children
	pub fn children_mut(&mut self) -> &mut [RsxNode] {
		match self {
			RsxNode::Fragment(rsx_nodes) => rsx_nodes,
			RsxNode::Component { .. } => &mut [],
			RsxNode::Block(RsxBlock { initial, .. }) => initial.children_mut(),
			RsxNode::Element(RsxElement { children, .. }) => children,
			RsxNode::Text(_) => &mut [],
			RsxNode::Comment(_) => &mut [],
			RsxNode::Doctype => &mut [],
		}
	}
	/// takes all the register_effect functions
	/// # Panics
	/// If the register function fails
	pub fn register_effects(&mut self) {
		RsxContext::visit_mut(self, |cx, node| match node {
			RsxNode::Block(RsxBlock { effect, .. }) => {
				effect.take().register(cx).unwrap();
			}
			RsxNode::Element(e) => {
				for a in &mut e.attributes {
					match a {
						RsxAttribute::Block { effect, .. } => {
							effect.take().register(cx).unwrap();
						}
						RsxAttribute::BlockValue { effect, .. } => {
							effect.take().register(cx).unwrap();
						}
						_ => {}
					}
				}
			}
			_ => {}
		});
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

	/// try to insert nodes into the first slot found,
	/// returning any nodes that were not inserted.
	/// If the slot is not a direct child, recursively search children.
	/// Components are not searched because they would steal the slot
	/// from next siblings.
	pub fn try_insert_slots(
		&mut self,
		name: &str,
		mut to_insert: Vec<Self>,
	) -> Option<Vec<Self>> {
		match self {
			RsxNode::Fragment(children) => {
				for node in children.iter_mut() {
					match node.try_insert_slots(name, to_insert) {
						Some(returned_nodes) => to_insert = returned_nodes,
						None => return None,
					}
				}
				Some(to_insert)
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
						element.children.extend(to_insert);
						return None;
					}
				}
				// if we didnt find the slot, recursively search children
				for child in &mut element.children {
					match child.try_insert_slots(name, to_insert) {
						Some(returned_nodes) => to_insert = returned_nodes,
						None => return None,
					}
				}
				Some(to_insert)
			}
			RsxNode::Component { .. } => {
				Some(to_insert)
				// dont recurse into component because it would steal the slot
				// from next siblings
			}
			RsxNode::Block(RsxBlock { initial, .. }) => {
				initial.try_insert_slots(name, to_insert)
			}
			RsxNode::Text(_) => Some(to_insert),
			RsxNode::Comment(_) => Some(to_insert),
			RsxNode::Doctype => Some(to_insert),
		}
	}
}


/// Representation of a rusty node.
///
/// ```
/// # use beet_rsx::as_beet::*;
/// let my_block = 3;
/// let el = rsx! { <div>{my_block}</div> };
/// ```
#[derive(Debug)]
pub struct RsxBlock {
	pub initial: Box<RsxNode>,
	pub effect: Effect,
}


/// A component is a struct that implements the [Component] trait.
/// When it is used in an `rsx!` macro it will be instantiated
/// with the [`Component::render`] method and any slot children.
#[derive(Debug)]
pub struct RsxComponent {
	pub tag: String,
	/// even key value attribute changes must be tracked
	/// because components are structs not elements
	pub tracker: Option<RustyTracker>,
	/// the node returned by [Component::render]
	pub node: Box<RsxNode>,
	/// the children passed in by this components parent:
	///
	/// `rsx! { <MyComponent>slot_children</MyComponent> }`
	pub slot_children: Box<RsxNode>,
}

/// Representation of an RsxElement
///
/// ```
/// # use beet_rsx::as_beet::*;
/// let el = rsx! { <div class="my-class">hello world</div> };
/// ```
#[derive(Debug)]
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

// #[derive(Debug, Clone, PartialEq)]
// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
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
		effect: Effect,
	},
	// kind of like a fragment, but for attributes
	Block {
		initial: Vec<RsxAttribute>,
		effect: Effect,
	},
}

impl AsRef<RsxNode> for &RsxNode {
	fn as_ref(&self) -> &RsxNode { *self }
}

impl AsMut<RsxNode> for &mut RsxNode {
	fn as_mut(&mut self) -> &mut RsxNode { *self }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn root_location() {
		let line = line!() + 1;
		let RsxRoot { location, .. } = rsx! { <div>hello world</div> };
		expect(location.file()).to_be("crates/beet_rsx/src/rsx/rsx_node.rs");
		expect(location.line()).to_be(line as usize);
		expect(location.col()).to_be(40);
	}
}
