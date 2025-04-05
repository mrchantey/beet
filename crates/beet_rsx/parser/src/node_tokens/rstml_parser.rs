use crate::prelude::*;
use quote::ToTokens;
use rstml::node::NodeName;
use syn::LitStr;
use syn::spanned::Spanned;

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

