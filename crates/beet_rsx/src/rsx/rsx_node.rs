use crate::prelude::*;
use strum_macros::AsRefStr;
use strum_macros::EnumDiscriminants;


#[derive(Debug, AsRefStr, EnumDiscriminants)]
pub enum RsxNode {
	/// a html doctype node
	Doctype {
		idx: RsxIdx,
	},
	/// a html comment node
	Comment {
		idx: RsxIdx,
		value: String,
	},
	/// a html text node
	Text {
		idx: RsxIdx,
		value: String,
	},
	/// a rust block that returns text
	Block(RsxBlock),
	/// A transparent node that simply contains children
	/// This may be deprecated in the future if no patterns
	/// require it. The RstmlToRsx could support it
	Fragment {
		idx: RsxIdx,
		nodes: Vec<RsxNode>,
	},
	/// a html element
	Element(RsxElement),
	Component(RsxComponent),
}

impl Default for RsxNode {
	fn default() -> Self {
		Self::Fragment {
			idx: RsxIdx::default(),
			nodes: Vec::new(),
		}
	}
}


impl RsxNode {
	/// Returns true if the node is an empty fragment,
	/// or if it is recursively a fragment with only empty fragments
	pub fn is_empty(&self) -> bool {
		match self {
			RsxNode::Fragment { nodes, .. } => {
				for node in nodes {
					if !node.is_empty() {
						return false;
					}
				}
				true
			}
			_ => false,
		}
	}
	/// ## Panics
	/// If the node is not an empty fragment
	pub fn assert_empty(&self) {
		if !self.is_empty() {
			panic!(
				"Expected empty fragment. Slot children must be empty before mapping to html, please call HtmlSlotsVisitor::apply\nreceived: {:#?}",
				self
			);
		}
	}

	pub fn discriminant(&self) -> RsxNodeDiscriminants { self.into() }
	/// helper method to kick off a visitor
	pub fn walk(&self, visitor: &mut impl RsxVisitor) {
		visitor.walk_node(self)
	}

	// this is too dangerous? so easy to expect the rsx idx to be unchanged?
	//
	// /// Add another node. If this node is a fragment it will be appended
	// /// to the end, otherwise a new fragment will be created with the
	// /// current node and the new node.
	// pub fn push(&mut self, node: RsxNode) {
	// 	match self {
	// 		RsxNode::Fragment { nodes, .. } => nodes.push(node),
	// 		_ => {
	// 			// let idx = self
	// 			let mut nodes = vec![std::mem::take(self)];
	// 			nodes.push(node);
	// 			*self = RsxNode::Fragment {
	// 				idx: RsxIdx::default(),
	// 				nodes,
	// 			};
	// 		}
	// 	}
	// }

	/// Returns true if the node is an html node
	pub fn is_html_node(&self) -> bool {
		match self {
			RsxNode::Doctype { .. }
			| RsxNode::Comment { .. }
			| RsxNode::Text { .. }
			| RsxNode::Element(_) => true,
			_ => false,
		}
	}

	/// takes all the register_effect functions
	/// # Panics
	/// If the register function fails
	pub fn register_effects(&mut self) {
		TreeLocationVisitor::visit_mut(self, |loc, node| {
			// println!(
			// 	"registering effect at loc: {:?}:{:?}",
			// 	loc,
			// 	node.discriminant()
			// );

			match node {
				RsxNode::Block(RsxBlock { effect, .. }) => {
					effect.take().register(loc).unwrap();
				}
				RsxNode::Element(e) => {
					for a in &mut e.attributes {
						match a {
							RsxAttribute::Block { effect, .. } => {
								effect.take().register(loc).unwrap();
							}
							RsxAttribute::BlockValue { effect, .. } => {
								effect.take().register(loc).unwrap();
							}
							_ => {}
						}
					}
				}
				_ => {}
			}
		});
	}

	/// non-recursive check for blocks in self, accounting for fragments
	pub fn directly_contains_rust_node(&self) -> bool {
		fn walk(node: &RsxNode) -> bool {
			match node {
				RsxNode::Block(_) => true,
				RsxNode::Fragment { nodes, .. } => {
					for item in nodes {
						if walk(item) {
							return true;
						}
					}
					false
				}
				_ => false,
			}
		}
		walk(self)
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
	pub idx: RsxIdx,
	/// The initial for an rsx block is considered a seperate tree,
	pub initial: Box<RsxRoot>,
	pub effect: Effect,
}
/// Representation of a rusty node.
///
/// ```
/// # use beet_rsx::as_beet::*;
/// let my_block = 3;
/// let el = rsx! { <div>{my_block}</div> };
/// ```
#[derive(Debug, Deref, DerefMut)]
pub struct RsxFragment(pub Vec<RsxNode>);

/// A component is a struct that implements the [Component] trait.
/// When it is used in an `rsx!` macro it will be instantiated
/// with the [`Component::render`] method and any slot children.
#[derive(Debug)]
pub struct RsxComponent {
	/// The index of this node in the local tree
	pub idx: RsxIdx,
	/// The name of the component, this must start with a capital letter
	pub tag: String,
	/// Tracks the <MyComponent ..> opening tag for this component
	/// even key value attribute changes must be tracked
	/// because components are structs not elements
	pub tracker: RustyTracker,
	/// the root returned by [Component::render]
	pub root: Box<RsxRoot>,
	// /// the children passed in by this component's parent:
	// ///
	// /// `rsx! { <MyComponent>slot_children</MyComponent> }`
	pub slot_children: Box<RsxNode>,
	pub template_directives: Vec<TemplateDirective>,
}

/// Attributes with a colon `:` are considered special client directives,
/// for example `client:load`
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TemplateDirective {
	/// The part before the colon
	pub prefix: String,
	/// The part after the colon
	pub suffix: String,
	/// The part after the equals sign, if any
	pub value: Option<String>,
}

impl TemplateDirective {
	/// Create a new template directive
	/// ## Panics
	/// If the key does not contain two parts split by a colon
	pub fn new(key: &str, value: Option<&str>) -> Self {
		let mut parts = key.split(':');
		let prefix = parts
			.next()
			.expect("expected colon prefix in template directive");
		let suffix = parts
			.next()
			.expect("expected colon suffix in template directive");
		Self {
			prefix: prefix.into(),
			suffix: suffix.into(),
			value: value.map(|v| v.into()),
		}
	}
}

/// Representation of an RsxElement
///
/// ```
/// # use beet_rsx::as_beet::*;
/// let el = rsx! { <div class="my-class">hello world</div> };
/// ```
#[derive(Debug)]
pub struct RsxElement {
	/// The index of this node in the local tree
	pub idx: RsxIdx,
	/// ie `div, span, input`
	pub tag: String,
	/// ie `class="my-class"`
	pub attributes: Vec<RsxAttribute>,
	/// ie `<div>childtext<childel/>{childblock}</div>`
	pub children: Box<RsxNode>,
	/// ie `<input/>`
	pub self_closing: bool,
}


impl RsxElement {
	/// Whether any children or attributes are blocks,
	/// used to determine whether the node requires an id
	pub fn contains_rust(&self) -> bool {
		self.children.directly_contains_rust_node()
			|| self.attributes.iter().any(|a| {
				matches!(
					a,
					RsxAttribute::Block { .. }
						| RsxAttribute::BlockValue { .. }
				)
			})
	}

	/// only checks [RsxAttribute::Key]
	pub fn contains_attr_key(&self, key: &str) -> bool {
		self.attributes.iter().any(|a| match a {
			RsxAttribute::Key { key: k } if k == key => true,
			_ => false,
		})
	}

	/// Try to find a matching value for a key
	pub fn get_key_value_attr(&self, key: &str) -> Option<&str> {
		self.attributes.iter().find_map(|a| match a {
			RsxAttribute::KeyValue { key: k, value } if k == key => {
				Some(value.as_str())
			}
			_ => None,
		})
	}

	/// Remove all attributes with the given key, checking:
	/// - [RsxAttribute::Key]
	/// - [RsxAttribute::KeyValue]
	/// - [RsxAttribute::BlockValue]
	pub fn remove_matching_key(&mut self, match_key: &str) {
		self.attributes.retain(|a| match a {
			RsxAttribute::Key { key } => key != match_key,
			RsxAttribute::KeyValue { key, .. } => key != match_key,
			RsxAttribute::BlockValue { key, .. } => key != match_key,
			_ => true,
		});
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
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[test]
	fn root_location() {
		let line = line!() + 1;
		let RsxRoot { location, .. } = rsx! { <div>hello world</div> };
		expect(location.file().replace("\\", "/"))
			.to_be("crates/beet_rsx/src/rsx/rsx_node.rs");
		expect(location.line()).to_be(line as usize);
		expect(location.col()).to_be(40);
	}
}
