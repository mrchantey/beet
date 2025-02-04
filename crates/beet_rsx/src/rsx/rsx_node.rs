use crate::prelude::*;
use anyhow::Result;
use strum_macros::AsRefStr;
use strum_macros::EnumDiscriminants;

/// File location of an rsx macro, used by [RsxTemplate]
/// to reconcile rsx nodes with html partials
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RsxLocation {
	/// in the macro this is set via file!(),
	/// in the cli its set via the file path,
	/// when setting this it must be in the same
	/// format as file!() would return
	pub file: String,
	pub line: usize,
	pub col: usize,
}
impl RsxLocation {
	pub fn new(file: impl Into<String>, line: usize, col: usize) -> Self {
		Self {
			file: file.into(),
			line,
			col,
		}
	}

	pub fn file(&self) -> &str { &self.file }
	pub fn line(&self) -> usize { self.line }
	pub fn col(&self) -> usize { self.col }
}

pub type RegisterEffect = Box<dyn FnOnce(&RsxContext) -> Result<()>>;
pub struct Effect {
	/// the function for registering the effect with
	/// its reactive framework
	pub register: RegisterEffect,
	/// the location of the effect in the rsx macro,
	/// this may or may not be populated depending
	/// on the settings of the parser
	pub location: Option<LineColumn>,
}

impl Effect {
	pub fn new(register: RegisterEffect, location: Option<LineColumn>) -> Self {
		Self { register, location }
	}

	/// call the FnOnce register func and replace it
	/// with an empty one.
	pub fn register_take(&mut self, cx: &RsxContext) -> Result<()> {
		let func = std::mem::replace(&mut self.register, Box::new(|_| Ok(())));
		func(cx)
	}
}

impl std::fmt::Debug for Effect {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Effect")
			.field("location", &self.location)
			.field("register", &std::any::type_name_of_val(&self.register))
			.finish()
	}
}

#[derive(Debug, AsRefStr, EnumDiscriminants)]
pub enum RsxNode {
	/// A transparent node that simply contains children
	/// This may be deprecated in the future if no patterns
	/// require it. The RstmlToRsx could support it
	Fragment(Vec<RsxNode>),
	Component {
		tag: String,
		node: Box<RsxNode>,
	},
	/// a rust block that returns text
	Block {
		initial: Box<RsxNode>,
		effect: Effect,
	},
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
			RsxNode::Block { initial, .. } => initial.children(),
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
			RsxNode::Block { initial, .. } => initial.children_mut(),
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
			RsxNode::Block { effect, .. } => {
				effect.register_take(cx).unwrap();
			}
			RsxNode::Element(e) => {
				for a in &mut e.attributes {
					match a {
						RsxAttribute::Block { effect, .. } => {
							effect.register_take(cx).unwrap();
						}
						RsxAttribute::BlockValue { effect, .. } => {
							effect.register_take(cx).unwrap();
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
			RsxNode::Block { initial, .. } => {
				initial.try_insert_slots(name, to_insert)
			}
			RsxNode::Text(_) => Some(to_insert),
			RsxNode::Comment(_) => Some(to_insert),
			RsxNode::Doctype => Some(to_insert),
		}
	}
}

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
		let RsxRoot { location, .. } = rsx! {<div>hello world</div>};
		expect(location.file()).to_be("crates/beet_rsx/src/rsx/rsx_node.rs");
		expect(location.line()).to_be(line as usize);
		expect(location.col()).to_be(39);
	}
	#[test]
	fn block_location() {
		fn get_hash(RsxRoot { node, .. }: RsxRoot) -> u64 {
			let RsxNode::Block { effect, .. } = &node else {
				panic!()
			};
			let Some(location) = &effect.location else {
				panic!()
			};
			location.to_hash()
		}
		#[rustfmt::skip]
		expect(get_hash(rsx! {{39}})).not().to_be(get_hash(rsx! {{39}}));
	}
}
