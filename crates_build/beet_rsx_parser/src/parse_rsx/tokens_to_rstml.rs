use proc_macro2::TokenStream;
use quote::quote;
use quote::quote_spanned;
use rstml::Parser;
use rstml::ParserConfig;
use rstml::node::CustomNode;
use rstml::node::Node;
use rstml::node::NodeName;
use std::collections::HashSet;
use sweet::prelude::Pipeline;
use syn::spanned::Spanned;

/// Converts token stream to [`rstml`] nodes and errors.
/// Pipeline: [`Pipeline<TokenStream, (Vec<Node<C>>, Vec<TokenStream>)>`]
pub struct TokensToRstml<C = rstml::Infallible> {
	self_closing_elements: HashSet<&'static str>,
	phantom: std::marker::PhantomData<C>,
}

impl<C> Default for TokensToRstml<C> {
	fn default() -> Self {
		Self {
			self_closing_elements: self_closing_elements(),
			phantom: std::marker::PhantomData,
		}
	}
}

impl TokensToRstml<rstml::Infallible> {
	/// convenience without specifying the type
	pub fn new() -> Self { Self::default() }
}


impl<C: CustomNode + std::fmt::Debug>
	Pipeline<TokenStream, (Vec<Node<C>>, Vec<TokenStream>)> for TokensToRstml<C>
{
	fn apply(self, tokens: TokenStream) -> (Vec<Node<C>>, Vec<TokenStream>) {
		let config = ParserConfig::new()
			.recover_block(true)
			.always_self_closed_elements(self.self_closing_elements)
			.raw_text_elements(["script", "style"].into_iter().collect())
			// here we define the rsx! as the constant thats used
			// to resolve raw text blocks more correctly
			.macro_call_pattern(quote!(rsx! {%%}))
			.custom_node::<C>();

		let parser = Parser::new(config);
		let (nodes, errors) = parser.parse_recoverable(tokens).split_vec();

		let errors = errors
			.into_iter()
			.map(|e| e.emit_as_expr_tokens())
			.collect();

		(nodes, errors)
	}
}


/// elements that are self-closing, rstml requires hashset
pub fn self_closing_elements() -> HashSet<&'static str> {
	[
		"area", "base", "br", "col", "embed", "hr", "img", "input", "link",
		"meta", "param", "source", "track", "wbr",
	]
	.into_iter()
	.collect()
}

fn _generate_tags_docs(elements: &[NodeName]) -> Vec<proc_macro2::TokenStream> {
	// Mark some of elements as type,
	// and other as elements as fn in crate::docs,
	// to give an example how to link tag with docs.
	let elements_as_type: HashSet<&'static str> =
		vec!["html", "head", "meta", "link", "body"]
			.into_iter()
			.collect();

	elements
		.into_iter()
		.map(|e| {
			if elements_as_type.contains(&*e.to_string()) {
				let element = quote_spanned!(e.span() => enum);
				quote!({#element X{}})
			} else {
				// let _ = crate::docs::element;
				let element = quote_spanned!(e.span() => element);
				quote!(let _ = crate::docs::#element)
			}
		})
		.collect()
}
