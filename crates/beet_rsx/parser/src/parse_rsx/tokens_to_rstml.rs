use proc_macro2::TokenStream;
use proc_macro2_diagnostics::Diagnostic;
use quote::quote;
use quote::quote_spanned;
use rstml::node::Node;
use rstml::node::NodeName;
use rstml::Parser;
use rstml::ParserConfig;
use std::collections::HashSet;
use syn::spanned::Spanned;

/// elements that are self-closing
pub fn self_closing_elements() -> HashSet<&'static str> {
	[
		"area", "base", "br", "col", "embed", "hr", "img", "input", "link",
		"meta", "param", "source", "track", "wbr",
	]
	.into_iter()
	.collect()
}


pub fn tokens_to_rstml(tokens: TokenStream) -> (Vec<Node>, Vec<Diagnostic>) {
	let empty_elements = self_closing_elements();
	let config = ParserConfig::new()
		.recover_block(true)
		.always_self_closed_elements(empty_elements)
		.raw_text_elements(["script", "style"].into_iter().collect())
		.macro_call_pattern(quote!(html! {%%}));

	let parser = Parser::new(config);
	let (nodes, errors) = parser.parse_recoverable(tokens).split_vec();

	(nodes, errors)
}

pub fn _generate_tags_docs(
	elements: &[NodeName],
) -> Vec<proc_macro2::TokenStream> {
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
