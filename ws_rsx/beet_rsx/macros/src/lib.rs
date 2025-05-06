use beet_rsx_parser::prelude::*;
use proc_macro::TokenStream;
use syn::DeriveInput;
use syn::parse_macro_input;
mod derive_deref;
use sweet::prelude::*;




/// This macro expands to an [WebNode](beet_rsx::prelude::WebNode).
/// ```ignore
/// let node = rsx! {<div> the value is {3}</div>};
/// ```
///
#[proc_macro]
pub fn rsx(tokens: TokenStream) -> TokenStream {
	tokens.xpipe(RsxMacroPipeline::default()).into()
}

/// Mostly used for testing,
/// this macro expands to an RsxTemplateNode, it is used for
/// things like hot reloading.
#[proc_macro]
pub fn rsx_template(tokens: TokenStream) -> TokenStream {
	tokens.xpipe(RsxTemplateMacroPipeline::default()).into()
}




#[proc_macro_derive(Deref)]
pub fn derive_deref(input: TokenStream) -> TokenStream {
	derive_deref::derive_deref(input)
}

#[proc_macro_derive(DerefMut)]
pub fn derive_deref_mut(input: TokenStream) -> TokenStream {
	derive_deref::derive_deref_mut(input)
}



/// Adds a builder pattern to a struct enabling construction as an
/// rsx component
#[proc_macro_derive(Node, attributes(node, field))]
pub fn derive_node(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	parse_derive_node(input).into()
}

/// Allow a struct to be included as a `#[field(flatten)]` of another struct
#[proc_macro_derive(Buildable, attributes(field))]
pub fn derive_buildable(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	parse_derive_buildable(input).into()
}
/// Implements [`IntoBlockAttribute`] for a struct.
/// Optional fields will checked and only added if they are Some.
/// All fields must implement Into<String>.
#[proc_macro_derive(IntoBlockAttribute, attributes(field))]
pub fn derive_into_block_attribute(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	impl_into_block_attribute(input).into()
}
