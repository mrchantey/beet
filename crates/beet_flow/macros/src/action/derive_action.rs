use super::ActionAttributes;
use crate::utils::CrateManifest;
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

pub fn impl_derive_action(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	let input = syn::parse_macro_input!(input as syn::DeriveInput);
	let result = parse(input);
	result.unwrap_or_else(|err| err.into_compile_error()).into()
}

fn parse(input: DeriveInput) -> syn::Result<TokenStream> {
	let attributes = ActionAttributes::parse(&input.attrs)?;

	let impl_component = impl_component(&input, &attributes);

	Ok(quote! {
		#impl_component
	})
}

fn impl_component(
	input: &DeriveInput,
	attributes: &ActionAttributes,
) -> TokenStream {
	let ident = &input.ident;
	let (impl_generics, type_generics, where_clause) =
		&input.generics.split_for_impl();

	let beet_flow_path = CrateManifest::get_path_direct("beet_flow");

	let storage = attributes.storage.clone().unwrap_or_else(|| {
		syn::parse_quote! { bevy::ecs::component::StorageType::Table }
	});

	let observers = attributes.observers.iter().map(|observer| {
		quote! {
			world.commands().entity(action).observe(#observer);
		}
	});


	quote! {
		impl #impl_generics bevy::prelude::Component for #ident #type_generics #where_clause {
			const STORAGE_TYPE: bevy::ecs::component::StorageType = #storage;
			// wrong as of 0.16
			fn register_component_hooks(
				hooks: &mut bevy::ecs::component::ComponentHooks,
			) {
				hooks.on_add(|mut world, node, cid| {
					#beet_flow_path::prelude::ActionObservers::on_add(&mut world, node, cid, |world, action| {
						#(#observers)*
					});
				});
				hooks.on_remove(|mut world, node, _cid| {
					#beet_flow_path::prelude::ActionObservers::on_remove(&mut world, node);
				});
			}
		}
	}
}
