use beet_core_shared::AttributeGroup;
use proc_macro2::TokenStream;
use quote::quote;
use syn;
use syn::DeriveInput;
use syn::parse_macro_input;


/// ActionEvent is now just an alias for EntityTargetEvent
pub fn impl_action_event(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	impl_entity_target_event(input)
}

pub fn impl_entity_target_event(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	parse_entity_target_event(input)
		.unwrap_or_else(|err| err.into_compile_error())
		.into()
}

fn parse_entity_target_event(input: DeriveInput) -> syn::Result<TokenStream> {
	let (impl_generics, type_generics, where_clause) =
		input.generics.split_for_impl();
	let input_ident = &input.ident;

	let attrs = AttributeGroup::parse(&input.attrs, "event")?;

	let auto_propagate = attrs.contains("auto_propagate");
	let default_propagate = quote::quote!(&'static ChildOf);
	let propagate = attrs.get_value("propagate").unwrap_or(&default_propagate);

	Ok(quote! {
		impl #impl_generics Event for #input_ident #type_generics #where_clause {
			type Trigger<'a> = EntityTargetTrigger<
				#auto_propagate,
				Self,
				#propagate
			>;
		}
	})
}



#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn works() {
		let result =
			parse_entity_target_event(syn::parse_quote! {struct MyEvent;})
				.unwrap()
				.to_string();

		let expected = "impl Event for MyEvent { type Trigger < 'a > = EntityTargetTrigger < false , Self , & 'static ChildOf > ; }";
		assert_eq!(expected, result);
	}
}
