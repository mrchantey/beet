use bevy::prelude::*;


/// Indicates a Html Doctype Node, [W3 Docs](https://www.w3schools.com/tags/tag_doctype.ASP)
#[derive(Component)]
pub struct DoctypeNode;

#[cfg(feature = "tokens")]
impl crate::prelude::RustTokens for DoctypeNode {
	fn into_rust_tokens(&self) -> proc_macro2::TokenStream {
		quote::quote! {DoctypeNode}
	}
}

/// Indicates a Html Comment Node, [W3 Docs](https://www.w3schools.com/tags/tag_comment.asp)
#[derive(Component)]
pub struct CommentNode(pub String);

/// Indicates a Html Element Node, [W3 Docs](https://www.w3schools.com/jsref/prop_node_nodetype.asp).
/// For the tag see [`NodeTag`].
#[derive(Component)]
pub struct ElementNode {
	pub self_closing: bool,
}
