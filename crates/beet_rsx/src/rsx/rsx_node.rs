use crate::prelude::*;
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


#[derive(Debug, Clone)]
pub struct RsxDoctype {
	pub location: Option<RsxMacroLocation>,
}


#[derive(Debug, Clone)]
pub struct RsxComment {
	pub value: String,
	pub location: Option<RsxMacroLocation>,
}

#[derive(Debug, Clone)]
pub struct RsxText {
	pub value: String,
	pub location: Option<RsxMacroLocation>,
}

#[derive(Debug, Default, Clone)]
pub struct RsxFragment {
	pub nodes: Vec<RsxNode>,
	pub location: Option<RsxMacroLocation>,
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

pub struct VecIntoRsx;
impl<T: IntoRsxNode<M2>, M2> IntoRsxNode<(M2, VecIntoRsx)> for Vec<T> {
	fn into_node(self) -> RsxNode {
		RsxFragment {
			nodes: self.into_iter().map(|item| item.into_node()).collect(),
			location: None,
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
			location: None,
		})
	}
}

impl RsxNode {
	#[rustfmt::skip]
	pub fn location(&self) -> Option<&RsxMacroLocation> {
		match self {
			RsxNode::Doctype(RsxDoctype { location, .. }) => location.as_ref(),
			RsxNode::Comment(RsxComment { location, .. }) => location.as_ref(),
			RsxNode::Text(RsxText { location, .. }) => location.as_ref(),
			RsxNode::Block(RsxBlock { location, .. }) => location.as_ref(),
			RsxNode::Fragment(RsxFragment { location, .. }) => location.as_ref(),
			RsxNode::Element(RsxElement { location, .. }) => location.as_ref(),
			RsxNode::Component(RsxComponent { location, .. }) => location.as_ref(),
		}
	}
	#[rustfmt::skip]
	pub fn with_location(mut self, location: RsxMacroLocation) -> Self {
		match &mut self {
			RsxNode::Doctype(RsxDoctype { location: loc, .. }) => *loc = Some(location),
			RsxNode::Comment(RsxComment { location: loc, .. }) => *loc = Some(location),
			RsxNode::Text(RsxText { location: loc, .. }) => *loc = Some(location),
			RsxNode::Block(RsxBlock { location: loc, .. }) => *loc = Some(location),
			RsxNode::Fragment(RsxFragment { location: loc, .. }) => *loc = Some(location),
			RsxNode::Element(RsxElement { location: loc, .. }) => *loc = Some(location),
			RsxNode::Component(RsxComponent { location: loc, .. }) => *loc = Some(location),
		}
		self
	}
	#[rustfmt::skip]
	pub fn remove_location(&mut self) -> &mut Self {
		match self {
			RsxNode::Doctype(RsxDoctype { location, .. }) => *location = None,
			RsxNode::Comment(RsxComment { location, .. }) => *location = None,
			RsxNode::Text(RsxText { location, .. }) => *location = None,
			RsxNode::Block(RsxBlock { location, .. }) => *location = None,
			RsxNode::Fragment(RsxFragment { location, .. }) => *location = None,
			RsxNode::Element(RsxElement { location, .. }) => *location = None,
			RsxNode::Component(RsxComponent { location, .. }) => *location = None,
		}
		self
	}

	pub fn location_str(&self) -> String {
		match self.location() {
			Some(loc) => loc.to_string(),
			None => "<unknown>".to_string(),
		}
	}

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
	/// The location of the block itsself, not that of its initial node
	pub location: Option<RsxMacroLocation>,
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
	/// Collected template directives
	pub template_directives: Vec<TemplateDirective>,
	/// The location of the node
	pub location: Option<RsxMacroLocation>,
}

/// Attributes with a colon `:` are considered special template directives,
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
	pub location: Option<RsxMacroLocation>,
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

pub trait IntoRsxAttributes<M> {
	/// Convert this into a RsxAttribute
	fn into_rsx_attributes(self) -> Vec<RsxAttribute>;
}

impl IntoRsxAttributes<Vec<RsxAttribute>> for Vec<RsxAttribute> {
	fn into_rsx_attributes(self) -> Vec<RsxAttribute> { self }
}

impl<F: FnOnce() -> Vec<RsxAttribute>> IntoRsxAttributes<F> for F {
	fn into_rsx_attributes(self) -> Vec<RsxAttribute> { self() }
}

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
			.to_be("crates/beet_rsx/src/rsx/rsx_node.rs");
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
				.bpipe(RsxToHtmlString::default())
				.unwrap(),
		)
		.to_be("<div data-beet-rsx-idx=\"2\">3</div>");
	}
	#[test]
	fn block_attr() {
		let value = vec![RsxAttribute::Key {
			key: "foo".to_string(),
		}];
		let _node = rsx! { <el {value} /> };
	}
}
