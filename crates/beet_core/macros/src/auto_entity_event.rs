use beet_utils::prelude::AttributeGroup;
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

	let attrs = AttributeGroup::parse(&input.attrs, "entity_event")?;

	let auto_propagate = attrs.contains("auto_propagate");
	// let default_propagate = syn::parse_quote!(&'static ChildOf);
	// let propagate = attrs.get_value("propagate").unwrap_or(&default_propagate);
	// #propagate


	Ok(quote! {
		impl #impl_generics Event for #input_ident #type_generics #where_clause {
			type Trigger<'a> = AutoEntityTrigger<
				#auto_propagate,
				Self,
				&'static ChildOf
			>;
		}
	})
}
