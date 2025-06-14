use proc_macro2::TokenStream;
use syn;
use syn::DeriveInput;
use syn::parse_macro_input;

pub fn impl_non_send_component(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	parse(input)
		.unwrap_or_else(|err| err.into_compile_error())
		.into()
}


fn parse(_input: DeriveInput) -> syn::Result<TokenStream> {
	unimplemented!(
		"stuck with 'extending' Component so require etc still works"
	);
}
