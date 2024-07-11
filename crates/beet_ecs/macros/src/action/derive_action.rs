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
	let impl_action_systems = impl_action_systems(&input, &attributes)?;
	let impl_action_meta = impl_action_meta(&input, &attributes)?;

	Ok(quote! {
		use #beet_ecs_path::prelude::*;
		use bevy::prelude::*;
		#impl_action_systems
		#impl_action_meta
	})
}

fn impl_component_hooks(
	input: &DeriveInput,
	attributes: &ActionAttributes,
) -> syn::Result<Option<TokenStream>> {
	let ident = &input.ident;
	let (_impl_generics, type_generics, _where_clause) =
		&input.generics.split_for_impl();

	if attributes.observers.len() == 0 {
		return Ok(None);
	}

	let observers = attributes.observers.collect_comma_punct();

	Ok(Some(quote! {
		app.world_mut().register_component_hooks::<#ident #type_generics>()
		.on_add(|mut world, entity, _| {
				ActionObserversBuilder::new::<#ident #type_generics>()
				.add_observers((#observers))
				.build(world.commands(), entity);
			})
		.on_remove(|mut world, entity, _|{
				ActionObserversBuilder::cleanup::<#ident #type_generics>(&mut world,entity);
			});
	}))
}




fn impl_action_systems(
	input: &DeriveInput,
	attributes: &ActionAttributes,
) -> syn::Result<TokenStream> {
	let ident = &input.ident;
	let (impl_generics, type_generics, where_clause) =
		&input.generics.split_for_impl();

	let add_systems = if attributes.systems.len() == 0 {
		quote! {}
	} else {
		let systems = attributes.systems.collect_comma_punct();
		quote! { config.add_systems(app, (#systems)); }
	};

	let component_hooks = impl_component_hooks(input, attributes)?;

	let add_global_observers = if attributes.global_observers.len() == 0 {
		quote! {}
	} else {
		let adds: TokenStream = attributes
			.global_observers
			.iter()
			.map(|obs| {
				quote! { world.observe(#obs); }
			})
			.collect();
		quote! {
			let world = app.world_mut();
			#adds
		}
	};

	Ok(quote! {
		impl #impl_generics ActionSystems for #ident #type_generics #where_clause {
			fn on_build(app: &mut App, config: &BeetConfig) {
				#add_systems
				#add_global_observers
				#component_hooks
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
