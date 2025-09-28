use proc_macro2::TokenStream;
use quote::quote;
use syn;
use syn::DeriveInput;
use syn::parse_macro_input;



pub fn auto_entity_event(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	parse(input)
		.unwrap_or_else(|err| err.into_compile_error())
		.into()
}

fn parse(input: DeriveInput) -> syn::Result<TokenStream> {
	let (impl_generics, type_generics, where_clause) =
		input.generics.split_for_impl();
	let input_ident = &input.ident;

	Ok(quote! {
		impl #impl_generics Event for #input_ident #type_generics #where_clause {
			type Trigger<'a> = AutoEntityTrigger<
				false,
				Self,
				&'static ChildOf,
			>;
		}
	})
}
