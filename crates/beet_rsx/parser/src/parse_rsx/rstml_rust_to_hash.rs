use super::tokens_to_rstml;
use proc_macro2::TokenStream;
use quote::ToTokens;
use rstml::node::NodeAttribute;
use rstml::visitor::visit_nodes;
use rstml::visitor::Visitor;
use std::hash::DefaultHasher;
use std::hash::Hash;


/// Hash all the 'rusty' parts of the rsx macro
pub struct RstmlRustToHash<'a> {
	hasher: &'a mut DefaultHasher,
}


impl<'a> RstmlRustToHash<'a> {
	/// visit and hash without validating the rsx
	pub fn hash(hasher: &'a mut DefaultHasher, tokens: TokenStream) {
		let (mut nodes, _) = tokens_to_rstml(tokens.clone());
		let this = Self { hasher };
		visit_nodes(&mut nodes, this);
	}
}
impl<'a> syn::visit_mut::VisitMut for RstmlRustToHash<'a> {}

impl<'a, C> Visitor<C> for RstmlRustToHash<'a>
where
	C: rstml::node::CustomNode + 'static,
{
	fn visit_block(&mut self, block: &mut rstml::node::NodeBlock) -> bool {
		block.to_token_stream().to_string().hash(self.hasher);
		false
	}
	fn visit_element(
		&mut self,
		element: &mut rstml::node::NodeElement<C>,
	) -> bool {
		// we must hash component open tags because if the keys change
		// thats also a recompile.
		if element
			.open_tag
			.name
			.to_string()
			.starts_with(|c: char| c.is_uppercase())
		{
			element
				.open_tag
				.to_token_stream()
				.to_string()
				.hash(self.hasher);
		}
		// visit children
		true
	}
	fn visit_attribute(&mut self, attribute: &mut NodeAttribute) -> bool {
		// attributes
		match attribute {
			NodeAttribute::Block(block) => {
				block.to_token_stream().to_string().hash(self.hasher);
			}
			NodeAttribute::Attribute(attr) => match attr.value() {
				Some(syn::Expr::Lit(_)) => {}
				Some(value) => {
					value.to_token_stream().to_string().hash(self.hasher)
				}
				None => {}
			},
		}
		false
	}
}
