use crate::prelude::*;
use bevy::prelude::*;

/// Indicates a Html Fragment Node, which has children but no tag
#[derive(Component)]
pub struct FragmentNode;

/// Indicates a Html Doctype Node, [W3 Docs](https://www.w3schools.com/tags/tag_doctype.ASP)
#[derive(Component)]
pub struct DoctypeNode;
/// Indicates a Html Comment Node, [W3 Docs](https://www.w3schools.com/tags/tag_comment.asp)
#[derive(Component)]
pub struct CommentNode {
	pub value: String,
}

/// Indicates a Html Text Node, [W3 Docs](https://www.w3schools.com/jsref/prop_node_nodetype.asp)
#[derive(Component)]
pub struct TextNode {
	pub value: String,
}
/// Indicates a Html Element Node, [W3 Docs](https://www.w3schools.com/jsref/prop_node_nodetype.asp)
#[derive(Component)]
pub struct ElementNode {
	pub tag: Spanner<String>,
	pub self_closing: bool,
}

/// A block of code that will resolve to a node
#[derive(Component)]
pub struct BlockNode {
	pub tracker: RustyTracker,
	#[cfg(feature = "tokens")]
	pub handle: NonSendHandle<syn::Expr>,
}

/// A node used for authoring, withoud a html representation
#[derive(Component)]
pub struct ComponentNode {
	pub tag: Spanner<String>,
	#[cfg(feature = "tokens")]
	/// used for generating rust tokens, this will only
	/// be `Some` if the node was generated from rust tokens.
	pub tag_span: Option<NonSendHandle<proc_macro2::Span>>,
	pub tracker: RustyTracker,
}

#[cfg(feature = "tokens")]
impl crate::prelude::RustTokens for WebDirective {
	fn into_rust_tokens(&self) -> proc_macro2::TokenStream {
		match self {
			WebDirective::StyleId { id } => {
				quote::quote! {WebDirective::StyleId{ id: #id }}
			}
		}
	}
}
