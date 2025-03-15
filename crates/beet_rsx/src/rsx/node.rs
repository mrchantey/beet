use crate::prelude::*;

/// Represents a node in the RSX tree.
/// This is usually implemented by the `Node` derive macro.
///
/// ## Example
/// ```
/// # use beet_rsx::prelude::*;
/// #[derive(Node)]
/// #[node(into_rsx)]
/// struct MyNode {
/// 	is_required: u32,
/// 	is_optional: Option<u32>,
/// 	#[field(default = 7)]
///   is_default: u32,
/// }
///
/// fn into_rsx(node:Node) -> RsxRoot {
///
/// }
/// ```
///
pub trait Node {
	/// The builder used by.
	type Builder: NodeBuilder<Node = Self>;
	/// A helper struct of bools used by the `rsx!`
	/// macro to determine that all required fields are present.
	type Required;

	fn into_rsx(self) -> RsxRoot;
}


pub trait NodeBuilder: Default {
	type Node: Node;
	fn build(self) -> Self::Node;
}
