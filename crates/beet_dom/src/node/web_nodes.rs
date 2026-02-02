//! Web-specific DOM node types.
//!
//! This module provides HTML-specific node types including doctype,
//! comment, and element nodes.

use crate::prelude::*;
use beet_core::prelude::*;

/// Specify types for variadic functions like TokenizeComponent
pub type WebNodes = (DoctypeNode, CommentNode, ElementNode, AttributeKey);


/// Indicates a Html Doctype Node, [W3 Docs](https://www.w3schools.com/tags/tag_doctype.ASP)
#[derive(Debug, Default, Copy, Clone, Component, Reflect)]
#[reflect(Default, Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct DoctypeNode;

/// Indicates a Html Comment Node, [W3 Docs](https://www.w3schools.com/tags/tag_comment.asp)
#[derive(Debug, Default, Clone, Deref, DerefMut, Component, Reflect)]
#[reflect(Default, Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct CommentNode(pub String);

impl CommentNode {
	/// Creates a new [`CommentNode`] with the given content.
	pub fn new(content: impl Into<String>) -> Self { Self(content.into()) }
}

/// Indicates a Html Element Node, [W3 Docs](https://www.w3schools.com/jsref/prop_node_nodetype.asp).
/// For the tag see [`NodeTag`].
#[derive(Debug, Default, Copy, Clone, Component, Reflect)]
#[reflect(Default, Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct ElementNode {
	/// Whether this element is self-closing (e.g., `<br/>`).
	pub self_closing: bool,
}
impl ElementNode {
	/// Creates a new [`ElementNode`] that is not self-closing.
	pub fn open() -> Self {
		Self {
			self_closing: false,
		}
	}
	/// Creates a new [`ElementNode`] that is self-closing.
	pub fn self_closing() -> Self { Self { self_closing: true } }
}
