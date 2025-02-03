use crate::prelude::*;
use strum_macros::AsRefStr;
use strum_macros::EnumDiscriminants;


/// File location of an rsx macro, used by [ReverseRsx]
/// to reconcile rsx nodes with html partials
#[derive(Debug, Clone)]
pub struct RsxLocation {
	/// in the macro this is set via file!(),
	/// in the cli its set via the file path,
	/// when setting this it must be in the same
	/// format as file!() would return
	file: String,
	line: usize,
	col: usize,
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



// TODO return result, this can certainly be fallible
pub type RegisterEffect = Box<dyn FnOnce(&RsxContext)>;


#[derive(AsRefStr, EnumDiscriminants)]
pub enum RsxNode {
	/// The root node of an rsx! macro.
	/// The location is used for [ReverseRsx]
	Root {
		nodes: Vec<RsxNode>,
		location: RsxLocation,
	},
	/// A transparent node that simply contains children
	Fragment(Vec<RsxNode>),
	Component {
		tag: String,
		node: Box<RsxNode>,
	},
	/// a rust block that returns text
	Block {
		initial: Box<RsxNode>,
		register_effect: RegisterEffect,
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

impl std::fmt::Debug for RsxNode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Root { nodes, location } => f
				.debug_struct("Root")
				.field("items", nodes)
				.field("location", location)
				.finish(),
			Self::Fragment(arg0) => {
				f.debug_tuple("Fragment").field(arg0).finish()
			}
			Self::Block {
				initial,
				register_effect,
			} => f
				.debug_struct("Block")
				.field("initial", initial)
				.field(
					"register_effect",
					&std::any::type_name_of_val(register_effect),
				)
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


	/// chidren of root, fragment or element.
	/// Blocks and components have no children
	pub fn children(&self) -> &[RsxNode] {
		match self {
			RsxNode::Root { nodes, .. } => nodes,
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
			RsxNode::Root { nodes, .. } => nodes,
			RsxNode::Fragment(rsx_nodes) => rsx_nodes,
			RsxNode::Component { .. } => &mut [],
			RsxNode::Block { initial, .. } => initial.children_mut(),
			RsxNode::Element(RsxElement { children, .. }) => children,
			RsxNode::Text(_) => &mut [],
			RsxNode::Comment(_) => &mut [],
			RsxNode::Doctype => &mut [],
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
			RsxNode::Root { nodes, .. } => {
				for node in nodes.iter_mut() {
					match node.try_insert_slots(name, to_insert) {
						Some(returned_nodes) => to_insert = returned_nodes,
						None => return None,
					}
				}
				Some(to_insert)
			}
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

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn location() {
		let node = rsx! {<div>hello world</div>};
		let RsxNode::Root { location, .. } = node else {
			panic!()
		};
		expect(location.file()).to_be("crates/beet_rsx/src/rsx/rsx_node.rs");
		expect(location.line()).to_be(269);
		expect(location.col()).to_be(19);
	}
}
