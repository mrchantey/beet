#[cfg(feature = "tokens")]
use crate::as_beet::*;
use bevy::prelude::*;

/// Specify types for variadic functions like TokenizeComponent
pub type RsxNodes = (
	NodeTag,
	FragmentNode,
	TemplateNode,
	TextNode,
	NumberNode,
	BoolNode,
	BlockNode,
);

/// The tag of a node, ie 'div' or 'MyTemplate'
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


/// A block of code that will resolve to a node.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Default, Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct BlockNode;


/// An entity with literal value, which can be a string, number, or boolean.
/// If the entity has an [`AttributeOf`] component, it is an attribute value,
/// otherwise it is a Text Node, [W3 Docs](https://www.w3schools.com/jsref/prop_node_nodetype.asp).

/// ## Display
/// The `Display` implementation is the [`ToString`] implementation for the inner value.
///
/// ## Hash
/// This type implements `Hash` including its f64 variant,
/// disregarding the fact that technically NaN is not equal to itself.
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	DerefMut,
	Component,
	Reflect,
)]
#[reflect(Default, Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[cfg_attr(feature = "bevy_default", require(bevy::prelude::TextSpan))]
pub struct TextNode(pub String);
impl TextNode {
	pub fn new(val: impl Into<String>) -> Self { Self(val.into()) }
}

/// A numerical [`TextNode`] with a f64 value, the two values are automatically kept in sync.
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	PartialOrd,
	Deref,
	DerefMut,
	Component,
	Reflect,
)]
#[reflect(Default, Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[cfg_attr(feature = "bevy_default", require(bevy::prelude::TextSpan))]
#[require(TextNode)]
pub struct NumberNode(pub f64);

impl NumberNode {
	pub fn new(value: impl Into<f64>) -> Self { Self(value.into()) }
}
/// A boolean [`TextNode`] with a f64 value, the two values are automatically kept in sync.
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	DerefMut,
	Component,
	Reflect,
)]
#[reflect(Default, Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[cfg_attr(feature = "bevy_default", require(bevy::prelude::TextSpan))]
#[require(TextNode)]
pub struct BoolNode(pub bool);
impl BoolNode {
	pub fn new(value: impl Into<bool>) -> Self { Self(value.into()) }
}


impl std::fmt::Display for TextNode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}
impl std::fmt::Display for NumberNode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}
impl std::fmt::Display for BoolNode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}
