use crate::as_beet::*;
use beet_common_macros::ToTokens;
use bevy::prelude::*;

define_token_collector!(
	CollectWebNodeTokens,
	doctypes: DoctypeNode,
	comments: CommentNode,
	elements: ElementNode,
);

/// Indicates a Html Doctype Node, [W3 Docs](https://www.w3schools.com/tags/tag_doctype.ASP)
#[derive(Debug, Default, Copy, Clone, Component, Reflect)]
#[reflect(Default, Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct DoctypeNode;

/// Indicates a Html Comment Node, [W3 Docs](https://www.w3schools.com/tags/tag_comment.asp)
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Default, Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct CommentNode(pub String);

/// Indicates a Html Element Node, [W3 Docs](https://www.w3schools.com/jsref/prop_node_nodetype.asp).
/// For the tag see [`NodeTag`].
#[derive(Debug, Default, Copy, Clone, Component, Reflect)]
#[reflect(Default, Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct ElementNode {
	pub self_closing: bool,
}
