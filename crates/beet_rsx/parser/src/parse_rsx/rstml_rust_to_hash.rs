use super::TokensToRstml;
use proc_macro2::TokenStream;
use quote::ToTokens;
use rapidhash::RapidHasher;
use rstml::node::CustomNode;
use rstml::node::NodeAttribute;
use rstml::visitor::visit_nodes;
use std::hash::Hash;
use sweet::prelude::PipelineTarget;


/// Hash all the 'rusty' parts of the rsx macro
pub struct RstmlRustToHash<'a, C = rstml::Infallible> {
	hasher: &'a mut RapidHasher,
	phantom: std::marker::PhantomData<C>,
}


impl<'a, C> RstmlRustToHash<'a, C> {
	fn hash(&mut self, tokens: impl ToTokens) {
		tokens
			.to_token_stream()
			.to_string()
			.replace(" ", "")
			.hash(self.hasher);
	}
}

impl<'a, C: 'static + CustomNode + std::fmt::Debug + Hash>
	RstmlRustToHash<'a, C>
{
	/// visit and hash without validating the rsx
	pub fn visit_and_hash(hasher: &'a mut RapidHasher, tokens: TokenStream) {
		let (mut nodes, _) = tokens.xpipe(TokensToRstml::<C>::default());
		let this = Self {
			hasher,
			phantom: std::marker::PhantomData,
		};
		visit_nodes(&mut nodes, this);
	}
}
impl<'a, C> syn::visit_mut::VisitMut for RstmlRustToHash<'a, C> {}

/// we could visit_rust_block but this feels more explicit
/// and easier to understand
impl<'a, C> rstml::visitor::Visitor<C> for RstmlRustToHash<'a, C>
where
	C: rstml::node::CustomNode,
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
