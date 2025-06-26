#[cfg(feature = "tokens")]
use crate::as_beet::*;
use bevy::prelude::*;

/// The tag of a node
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect, Deref, DerefMut)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct NodeTag(pub String);

impl std::fmt::Display for NodeTag {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl NodeTag {
	/// Create a new tag from a string
	pub fn new(tag: impl Into<String>) -> Self { Self(tag.into()) }
	/// Get the tag as a string slice
	pub fn tag(&self) -> &str { &self.0 }
}

/// aka `Component` in web, applied to nodes that are constructed using a builder pattern,
/// denoted as uppercase in rsx `<MyTemplate/>`.
/// These are represented in the entity graph as a [`FragmentNode`], ie they
/// have children but no visual representation. This allows templates themselves
/// to have components.
#[derive(Debug, Default, Copy, Clone, Component, Reflect)]
#[reflect(Default, Component)]
#[component(immutable)]
#[require(FragmentNode)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct TemplateNode;

/// Applied to non-visual nodes with children, this component enables
/// querying for entities that *are* a node but *are not* an element.
/// Every [`TemplateNode`] is a [`FragmentNode`]
#[derive(Debug, Default, Copy, Clone, Component, Reflect)]
#[reflect(Default, Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct FragmentNode;


/// Indicates a Html Text Node, [W3 Docs](https://www.w3schools.com/jsref/prop_node_nodetype.asp).
/// Note that adjacent text nodes are collapsed when rendering to html.
#[derive(Debug, Default, Clone, Component, Reflect, Deref, DerefMut)]
#[reflect(Default, Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct TextNode(pub String);

impl TextNode {
	pub fn new(text: impl Into<String>) -> Self { Self(text.into()) }
	pub fn text(&self) -> &str { &self.0 }
}

/// A block of code that will resolve to a node.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Default, Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct BlockNode;




