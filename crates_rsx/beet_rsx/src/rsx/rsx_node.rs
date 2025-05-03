use crate::prelude::*;
use anyhow::Result;
use strum_macros::AsRefStr;
use strum_macros::EnumDiscriminants;

#[derive(Debug, Clone, AsRefStr, EnumDiscriminants)]
pub enum RsxNode {
	/// a html doctype node
	Doctype(RsxDoctype),
	/// a html comment node
	Comment(RsxComment),
	/// a html text node
	Text(RsxText),
	/// a rust block that returns text
	Block(RsxBlock),
	/// A transparent node that simply contains children
	/// This may be deprecated in the future if no patterns
	/// require it. The RstmlToRsx could support it
	Fragment(RsxFragment),
	/// a html element
	Element(RsxElement),
	Component(RsxComponent),
}
impl Into<RsxNode> for RsxDoctype {
	fn into(self) -> RsxNode { RsxNode::Doctype(self) }
}
impl Into<RsxNode> for RsxComment {
	fn into(self) -> RsxNode { RsxNode::Comment(self) }
}
impl Into<RsxNode> for RsxText {
	fn into(self) -> RsxNode { RsxNode::Text(self) }
}
impl Into<RsxNode> for RsxBlock {
	fn into(self) -> RsxNode { RsxNode::Block(self) }
}
impl Into<RsxNode> for RsxFragment {
	fn into(self) -> RsxNode { RsxNode::Fragment(self) }
}
impl Into<RsxNode> for RsxElement {
	fn into(self) -> RsxNode { RsxNode::Element(self) }
}
impl Into<RsxNode> for RsxComponent {
	fn into(self) -> RsxNode { RsxNode::Component(self) }
}
impl NodeMeta for RsxNode {
	fn meta(&self) -> &RsxNodeMeta {
		match self {
			RsxNode::Doctype(node) => node.meta(),
			RsxNode::Comment(node) => node.meta(),
			RsxNode::Text(node) => node.meta(),
			RsxNode::Block(node) => node.meta(),
			RsxNode::Fragment(node) => node.meta(),
			RsxNode::Element(node) => node.meta(),
			RsxNode::Component(node) => node.meta(),
		}
	}

	fn meta_mut(&mut self) -> &mut RsxNodeMeta {
		match self {
			RsxNode::Doctype(node) => node.meta_mut(),
			RsxNode::Comment(node) => node.meta_mut(),
			RsxNode::Text(node) => node.meta_mut(),
			RsxNode::Block(node) => node.meta_mut(),
			RsxNode::Fragment(node) => node.meta_mut(),
			RsxNode::Element(node) => node.meta_mut(),
			RsxNode::Component(node) => node.meta_mut(),
		}
	}
}

#[derive(Debug, Clone)]
pub struct RsxDoctype {
	/// Metadata for this node
	pub meta: RsxNodeMeta,
}

impl NodeMeta for RsxDoctype {
	fn meta(&self) -> &RsxNodeMeta { &self.meta }
	fn meta_mut(&mut self) -> &mut RsxNodeMeta { &mut self.meta }
}

#[derive(Debug, Clone)]
pub struct RsxComment {
	pub value: String,
	/// Metadata for this node
	pub meta: RsxNodeMeta,
}

impl NodeMeta for RsxComment {
	fn meta(&self) -> &RsxNodeMeta { &self.meta }
	fn meta_mut(&mut self) -> &mut RsxNodeMeta { &mut self.meta }
}

#[derive(Debug, Clone)]
pub struct RsxText {
	pub value: String,
	/// Metadata for this node
	pub meta: RsxNodeMeta,
}


impl NodeMeta for RsxText {
	fn meta(&self) -> &RsxNodeMeta { &self.meta }
	fn meta_mut(&mut self) -> &mut RsxNodeMeta { &mut self.meta }
}

#[derive(Debug, Default, Clone)]
pub struct RsxFragment {
	pub nodes: Vec<RsxNode>,
	/// Metadata for this node
	pub meta: RsxNodeMeta,
}

impl NodeMeta for RsxFragment {
	fn meta(&self) -> &RsxNodeMeta { &self.meta }
	fn meta_mut(&mut self) -> &mut RsxNodeMeta { &mut self.meta }
}

/// This is an RsxNode and a location, which is required for hydration.
///
/// It is allowed for the [`RsxRoot`] to be default(), which means that
/// the macro location is a placeholder, this means that the the node
/// will not be eligible for nested templating etc. which is the case
/// anyway for Strings and ().
///
/// The struct returned from an rsx! macro.

pub trait IntoRsxNode<M = ()> {
	fn into_node(self) -> RsxNode;
}

pub struct IntoIntoRsx;
impl<T: Into<RsxNode>> IntoRsxNode<IntoIntoRsx> for T {
	fn into_node(self) -> RsxNode { self.into() }
}

impl IntoRsxNode<()> for () {
	fn into_node(self) -> RsxNode { RsxNode::default() }
}
pub struct FuncIntoRsx;
impl<T: FnOnce() -> U, U: IntoRsxNode<M2>, M2> IntoRsxNode<(M2, FuncIntoRsx)>
	for T
{
	fn into_node(self) -> RsxNode { self().into_node() }
}

pub struct IterIntoRsx;
impl<I, T, M2> IntoRsxNode<(M2, IterIntoRsx)> for I
where
	I: IntoIterator<Item = T>,
	T: IntoRsxNode<M2>,
{
	fn into_node(self) -> RsxNode {
		RsxFragment {
			nodes: self.into_iter().map(|item| item.into_node()).collect(),
			meta: RsxNodeMeta::default(),
		}
		.into()
	}
}



impl Default for RsxNode {
	fn default() -> Self { Self::Fragment(RsxFragment::default()) }
}

impl AsRef<RsxNode> for RsxNode {
	fn as_ref(&self) -> &RsxNode { self }
}

impl AsMut<RsxNode> for RsxNode {
	fn as_mut(&mut self) -> &mut RsxNode { self }
}

impl<T: ToString> From<T> for RsxNode {
	fn from(value: T) -> Self {
		RsxNode::Text(RsxText {
			value: value.to_string(),
			meta: RsxNodeMeta::default(),
		})
	}
}

// pub struct PathIntoRsx;
// impl IntoRsxNode<PathIntoRsx> for &Path {
// 	fn into_node(self) -> RsxNode {
// 		RsxNode::Text(RsxText {
// 			value: self.to_string_lossy().to_string(),
// 			meta: RsxNodeMeta::default(),
// 		})
// 	}
// }



impl RsxNode {
	/// Returns true if the node is an empty fragment,
	/// or if it is recursively a fragment with only empty fragments
	pub fn is_empty(&self) -> bool {
		match self {
			RsxNode::Fragment(fragment) => {
				for node in &fragment.nodes {
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

	/// Add another node. If this node is a fragment it will be appended
	/// to the end, otherwise a new fragment will be created with the
	/// current node and the new node.
	pub fn push(&mut self, node: RsxNode) {
		match self {
			RsxNode::Fragment(RsxFragment { nodes, .. }) => nodes.push(node),
			_ => {
				let mut nodes = vec![std::mem::take(self)];
				nodes.push(node);
				*self = RsxFragment {
					nodes,
					..Default::default()
				}
				.into();
			}
		}
	}

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


	/// non-recursive check for blocks in self, accounting for fragments
	pub fn directly_contains_rust_node(&self) -> bool {
		fn walk(node: &RsxNode) -> bool {
			match node {
				RsxNode::Block(_) => true,
				RsxNode::Fragment(fragment) => {
					for item in &fragment.nodes {
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
#[derive(Debug, Clone)]
pub struct RsxBlock {
	/// The initial for an rsx block is considered a seperate tree,
	pub initial: Box<RsxNode>,
	pub effect: Effect,
	/// Metadata for this node
	pub meta: RsxNodeMeta,
}

impl NodeMeta for RsxBlock {
	fn meta(&self) -> &RsxNodeMeta { &self.meta }
	fn meta_mut(&mut self) -> &mut RsxNodeMeta { &mut self.meta }
}

/// A component is a struct that implements the [Component] trait.
/// When it is used in an `rsx!` macro it will be instantiated
/// with the [`Component::render`] method and any slot children.
#[derive(Debug, Clone)]
pub struct RsxComponent {
	/// The name of the component, this must start with a capital letter
	pub tag: String,
	/// The type name extracted via [`std::any::type_name`]
	pub type_name: String,
	/// The serialized component, only `Some` if the component has a `client` directive
	pub ron: Option<String>,
	/// Tracks the <MyComponent ..> opening tag for this component
	/// even key value attribute changes must be tracked
	/// because components are structs not elements
	pub tracker: RustyTracker,
	/// the node returned by [Component::render]
	pub node: Box<RsxNode>,
	/// the children passed in by this component's parent:
	///
	/// `rsx! { <MyComponent>slot_children</MyComponent> }`
	pub slot_children: Box<RsxNode>,
	/// Metadata for this node
	pub meta: RsxNodeMeta,
}

impl NodeMeta for RsxComponent {
	fn meta(&self) -> &RsxNodeMeta { &self.meta }
	fn meta_mut(&mut self) -> &mut RsxNodeMeta { &mut self.meta }
}

/// Representation of an RsxElement
///
/// ```
/// # use beet_rsx::as_beet::*;
/// let el = rsx! { <div class="my-class">hello world</div> };
/// ```
#[derive(Debug, Clone)]
pub struct RsxElement {
	/// ie `div, span, input`
	pub tag: String,
	/// ie `class="my-class"`
	pub attributes: Vec<RsxAttribute>,
	/// ie `<div>childtext<childel/>{childblock}</div>`
	pub children: Box<RsxNode>,
	/// ie `<input/>`
	pub self_closing: bool,
	/// The location of the node
	pub meta: RsxNodeMeta,
}

impl NodeMeta for RsxElement {
	fn meta(&self) -> &RsxNodeMeta { &self.meta }
	fn meta_mut(&mut self) -> &mut RsxNodeMeta { &mut self.meta }
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
#[derive(Debug, Clone)]
pub enum RsxAttribute {
	Key {
		key: String,
	},
	KeyValue {
		key: String,
		value: String,
	},
	/// An attribute where the value is a 'block', could be represented
	/// as an identifier for a variable or a block:
	/// `key=value` or `key={value}` are both considered blocks.
	BlockValue {
		key: String,
		initial: String,
		effect: Effect,
	},
	/// Block attributes are a bit like fragments, but for attributes.
	/// A common use-case is prop flattening where a `Button` component
	/// has a  `ButtonHtmlAttributes` struct to directly apply to its
	/// element.
	Block {
		/// The initial key or key-value attributes
		/// for this block.
		/// Events will be a key that starts with `on`,
		/// and will have no value
		initial: Vec<(String, Option<String>)>,
		/// This effect will register all required
		/// dynamic parts of this block
		effect: Effect,
	},
}

pub struct AsStrIntoRsxAttributeMarker;
impl<T: Into<String>> IntoRsxAttribute<AsStrIntoRsxAttributeMarker> for T {
	fn into_rsx_attribute(self) -> RsxAttribute {
		RsxAttribute::Key { key: self.into() }
	}
}
pub trait IntoRsxAttribute<M> {
	/// Convert this into a RsxAttribute
	fn into_rsx_attribute(self) -> RsxAttribute;
}

/// A trait for types that can be converted into an [`RsxAttribute::Block`],
/// Unlike [`IntoRsxAttribute`] the creation must be split into parts
/// to allow for prop flattening.
pub trait IntoBlockAttribute<M>: 'static + Send + Sync {
	/// Sets the [`RsxAttribute::Block::initial`] value
	fn initial_attributes(&self) -> Vec<(String, Option<String>)>;
	/// Called by the [`RsxAttribute::Block::effect`], can also
	/// be called recursively on children to handle prop flattening
	fn register_effects(self, loc: TreeLocation) -> Result<()>;
}

// pub struct IntoVecIntoRsxAttributeMarker;
// impl<T: Into<Vec<RsxAttribute>>>
// 	IntoBlockAttribute<IntoVecIntoRsxAttributeMarker> for T
// {
// 	fn into_initial_attributes(self) -> Vec<RsxAttribute> { self.into() }

// 	fn register_effects
// }

// impl<F: FnOnce() -> Vec<RsxAttribute>> IntoBlockAttribute<F> for F {
// 	fn into_initial_attributes(self) -> Vec<RsxAttribute> { self() }
// }

#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[test]
	fn root_location() {
		let line = line!() + 2;
		#[rustfmt::skip]
		let location = rsx! { <div>hello world</div> }
			.location()
			.cloned()
			.unwrap();
		expect(&location.file().to_string_lossy())
			.to_be("crates_rsx/beet_rsx/src/rsx/rsx_node.rs");
		expect(location.line()).to_be(line);
		expect(location.col()).to_be(24);
	}

	#[derive(Node)]
	struct MyComponent {
		key: u32,
	}
	fn my_component(props: MyComponent) -> RsxNode {
		rsx! { <div>{props.key}</div> }
	}


	#[test]
	fn comp_attr() {
		let my_comp = MyComponent { key: 3 };
		expect(
			rsx! { <MyComponent {my_comp} /> }
				.xpipe(RsxToHtmlString::default())
				.unwrap(),
		)
		.to_be("<div data-beet-rsx-idx=\"2\">3</div>");
	}
	// #[test]
	// fn block_attr() {
	// 	let value = vec![RsxAttribute::Key {
	// 		key: "foo".to_string(),
	// 	}];
	// 	let _node = rsx! { <el {value} /> };
	// }
}
