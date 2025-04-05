use crate::prelude::*;
use proc_macro2::TokenStream;
use quote::ToTokens;
use rstml::Infallible;
use rstml::node::CustomNode;
use rstml::node::NodeName;
use std::fmt::Debug;
use std::ops::ControlFlow;
use syn::LitStr;
use syn::spanned::Spanned;

pub trait RstmlParser: Sized {
	type NodeTokens: CustomNodeTokens<RstmlParser = Self>;
	type CustomRstmlNode: CustomNode + Debug = Infallible;
	/// return any errors that were generated during parsing
	fn into_errors(self) -> Vec<TokenStream> { Default::default() }
	fn map_node(
		&mut self,
		node: RstmlNode<Self::CustomRstmlNode>,
	) -> ControlFlow<
		NodeTokens<Self::NodeTokens>,
		RstmlNode<Self::CustomRstmlNode>,
	> {
		ControlFlow::Continue(node)
	}
}

impl RstmlParser for () {
	type NodeTokens = ();
}

/// Simplifies the [`NodeName::Punctuated`] to a string literal
impl From<NodeName> for NameExpr {
	fn from(value: NodeName) -> Self {
		match value {
			NodeName::Path(expr_path) => NameExpr::ExprPath(expr_path.into()),
			NodeName::Punctuated(punctuated) => {
				let str: LitStr = LitStr::new(
					&punctuated.to_token_stream().to_string(),
					punctuated.span(),
				);
				NameExpr::String(str.into())
			}
			NodeName::Block(block) => NameExpr::Block(block.into()),
		}
	}
}
