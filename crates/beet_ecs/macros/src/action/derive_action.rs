use super::ActionAttributes;
use crate::utils::*;
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
	let impl_component = impl_component(&input, &attributes)?;
	let impl_action_systems = impl_action_systems(&input, &attributes)?;
	let impl_action_meta = impl_action_meta(&input, &attributes)?;

	Ok(quote! {
		use #beet_ecs_path::prelude::*;
		use bevy::prelude::*;
		#impl_component
		#impl_action_systems
		#impl_action_meta
	})
}

fn impl_component(
	input: &DeriveInput,
	attributes: &ActionAttributes,
) -> syn::Result<TokenStream> {
	let ident = &input.ident;
	let (impl_generics, type_generics, where_clause) =
		&input.generics.split_for_impl();
	let observers = build_generic_funcs(
		&attributes.observers_non_generic,
		&attributes.observers_generic,
		type_generics,
	);


	let (observers_on_add, observers_on_remove) = if observers.len() == 0 {
		(TokenStream::new(), TokenStream::new())
	} else {
		let observers = observers.collect_comma_punct();
		(
			quote! {
				ActionObservers::new::<#ident #type_generics>()
				.add_observers((#observers))
				.build(world.commands(), entity);
			},
			quote! {
				ActionObservers::cleanup::<#ident #type_generics>(&mut world,entity);
			},
		)
	};


	Ok(quote! {
		use bevy::ecs::component::ComponentHooks;
		use bevy::ecs::component::StorageType;
		impl #impl_generics Component for #ident #type_generics #where_clause {
			const STORAGE_TYPE: StorageType = StorageType::Table;
			#[allow(unused)]
			fn register_component_hooks(hooks: &mut ComponentHooks) {
				#[allow(unused)]
				hooks.on_add(|mut world, entity, _| {
					#observers_on_add
				});
				#[allow(unused)]
				hooks.on_remove(|mut world, entity, _|
				{
						#observers_on_remove
				});
			}
		}
	})
}




fn impl_action_systems(
	input: &DeriveInput,
	attributes: &ActionAttributes,
) -> syn::Result<TokenStream> {
	let ident = &input.ident;
	let (impl_generics, type_generics, where_clause) =
		&input.generics.split_for_impl();

	let systems = build_generic_funcs(
		&attributes.systems_non_generic,
		&attributes.systems_generic,
		type_generics,
	);

	let add_systems = if systems.len() == 0 {
		quote! {}
	} else {
		let systems = systems.collect_comma_punct();
		quote! { config.add_systems(app, (#systems)); }
	};

	Ok(quote! {
		impl #impl_generics ActionSystems for #ident #type_generics #where_clause {
			fn on_build(app: &mut App, config: &BeetConfig) {
				#add_systems
			}
		}
	})
}
fn impl_action_meta(
	input: &DeriveInput,
	attributes: &ActionAttributes,
) -> syn::Result<TokenStream> {
	let ident = &input.ident;
	let (impl_generics, type_generics, where_clause) =
		&input.generics.split_for_impl();

	let fn_category = attributes.category.as_ref().map(|c| {
		quote! {
			fn category(&self) -> ActionCategory { #c }
		}
	});

	Ok(quote! {
		impl #impl_generics ActionMeta for #ident #type_generics #where_clause {
			#fn_category
		}
	})
}
