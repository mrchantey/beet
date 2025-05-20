use crate::as_beet::*;
use bevy::prelude::*;

/// Platform agnostic node types used for various target platforms,
/// ie server, html, bevy_render. This type is only for helper methods
pub enum RsxNode {}

define_token_collector!(
	CollectRsxNodeTokens,
	node_tags: NodeTag,
	fragments: FragmentNode,
	texts: TextNode,
	blocks: BlockNode,
);


/// The tag of a node
#[derive(Debug, Clone, Component, Reflect, Deref, DerefMut)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct NodeTag(pub String);

/// Applied to non-visual nodes with children.
/// A web-dev 'Component' is also a fragment in beet, in which case a [`NodeTag`]
/// is used to determine the actual component.
#[derive(Debug, Default, Copy, Clone, Component, Reflect)]
#[reflect(Default, Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct FragmentNode;


/// Indicates a Html Text Node, [W3 Docs](https://www.w3schools.com/jsref/prop_node_nodetype.asp)
#[derive(Debug, Default, Clone, Component, Reflect, Deref, DerefMut)]
#[reflect(Default, Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct TextNode(pub String);

/// A block of code that will resolve to a node.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Default, Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct BlockNode;
