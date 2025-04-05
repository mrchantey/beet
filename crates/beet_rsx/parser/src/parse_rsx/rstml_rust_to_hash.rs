use super::tokens_to_rstml;
use proc_macro2::TokenStream;
use quote::ToTokens;
use rapidhash::RapidHasher;
use rstml::node::NodeAttribute;
use rstml::visitor::Visitor;
use rstml::visitor::visit_nodes;
use std::hash::Hash;


/// Hash all the 'rusty' parts of the rsx macro
pub struct RstmlRustToHash<'a> {
	hasher: &'a mut RapidHasher,
}



impl<'a> RstmlRustToHash<'a> {
	/// visit and hash without validating the rsx
	pub fn visit_and_hash(hasher: &'a mut RapidHasher, tokens: TokenStream) {
		let (mut nodes, _) =
			tokens_to_rstml::<rstml::Infallible>(tokens.clone());
		let this = Self { hasher };
		visit_nodes(&mut nodes, this);
	}
	fn hash(&mut self, tokens: impl ToTokens) {
		tokens
			.to_token_stream()
			.to_string()
			.replace(" ", "")
			.hash(self.hasher);
	}
}
impl<'a> syn::visit_mut::VisitMut for RstmlRustToHash<'a> {}

/// we could visit_rust_block but this feels more explicit
/// and easier to understand
impl<'a, C> Visitor<C> for RstmlRustToHash<'a>
where
	C: rstml::node::CustomNode + 'static,
{
	fn visit_block(&mut self, block: &mut rstml::node::NodeBlock) -> bool {
		// println!("visiting block: {}", block.into_token_stream().to_string());
		self.hash(block);
		true
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
			self.hash(&element.open_tag);
		}
		// visit children
		true
	}
	fn visit_attribute(&mut self, attribute: &mut NodeAttribute) -> bool {
		// attributes
		match attribute {
			NodeAttribute::Block(block) => {
				self.hash(block);
			}
			NodeAttribute::Attribute(attr) => match attr.value() {
				Some(syn::Expr::Lit(_)) => {}
				Some(value) => {
					self.hash(value);
				}
				None => {}
			},
		}
		true
	}
}
