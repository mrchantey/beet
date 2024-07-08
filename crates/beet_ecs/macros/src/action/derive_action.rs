use super::ActionAttributes;
use crate::utils::BeetManifest;
use crate::utils::TokenStreamVecExt;
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;


pub fn derive_action(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	let input = syn::parse_macro_input!(input as syn::DeriveInput);
	let result = parse(input);
	result.unwrap_or_else(|err| err.into_compile_error()).into()
}

fn parse(input: DeriveInput) -> syn::Result<TokenStream> {
	let attributes = ActionAttributes::parse(&input.attrs)?;

	let beet_ecs_path = BeetManifest::get_path_direct("beet_ecs");

	let ident = input.ident;
	let (impl_generics, type_generics, where_clause) =
		&input.generics.split_for_impl();
	let mut observers = attributes
		.observers_generic
		.into_iter()
		.map(|ident| {
			quote! { #ident::#type_generics }
		})
		.collect::<Vec<_>>();
	observers.extend(attributes.observers_non_generic.into_iter().map(
		|ident| {
			quote! { #ident }
		},
	));
	let observers = observers.collect_comma_punct();

	Ok(quote! {
		use #beet_ecs_path::prelude::*;
		use bevy::prelude::*;
		use bevy::ecs::component::ComponentHooks;
		use bevy::ecs::component::StorageType;
		impl #impl_generics Component for #ident #type_generics #where_clause {
			const STORAGE_TYPE: StorageType = StorageType::Table;
			fn register_component_hooks(hooks: &mut ComponentHooks) {
				hooks.on_add(|mut world, entity, _| {
					ActionObserverHooks::new::<#ident #type_generics>()
						.add_observers((#observers))
						.build(world.commands(), entity);
				});
				hooks.on_remove(ActionObserverHooks::cleanup::<#ident #type_generics>);
			}
		}
	})
}
