use crate::utils::*;
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use syn::Expr;
use syn::ItemStruct;
use syn::Result;


pub fn parse_action(
	attr: proc_macro::TokenStream,
	item: proc_macro::TokenStream,
) -> Result<TokenStream> {
	let mut input = syn::parse::<ItemStruct>(item)?;
	let args = &attributes_map(attr.into(), Some(&["system"]))?;

	let action_trait = action_trait(&input, args);

	remove_field_attributes(&mut input);

	Ok(quote! {
		use beet_ecs::prelude::*;
		use beet_ecs::exports::*;
		// #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Component)]
		#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Component, FieldUi)]
		#input
		#action_trait
	})
}

fn remove_field_attributes(input: &mut ItemStruct) {
	let attrs_to_remove = ["shared"];

	for field in input.fields.iter_mut() {
		field.attrs = field
			.attrs
			.clone()
			.into_iter()
			.filter(|attr| {
				!attrs_to_remove
					.iter()
					.any(|&name| attr.path().is_ident(name))
			})
			.collect();
	}
}

fn action_trait(
	input: &ItemStruct,
	args: &HashMap<String, Option<Expr>>,
) -> TokenStream {
	let ident = &input.ident;

	let meta = meta(input);
	let spawn = spawn(input);
	let tick_system = tick_system(args);
	let post_tick_system = post_tick_system(input);

	quote! {
		impl Action for #ident {
			fn duplicate(&self) -> Box<dyn Action> {
				Box::new(self.clone())
			}
			#meta

			#spawn

			#tick_system
			#post_tick_system
		}
	}
}

static ACTION_ID: AtomicUsize = AtomicUsize::new(0);


fn meta(input: &ItemStruct) -> TokenStream {
	let ident = &input.ident;
	let name = ident.to_string();
	let action_id = ACTION_ID.fetch_add(1, Ordering::SeqCst);

	quote! {
		fn meta(&self) -> ActionMeta {
			ActionMeta {
				id: #action_id,
				name: #name
			}
		}
	}
}

fn tick_system(args: &HashMap<String, Option<Expr>>) -> TokenStream {
	let expr = args.get("system").unwrap().as_ref().unwrap();
	quote! {
		fn tick_system(&self) -> SystemConfigs {
			#expr.into_configs()
		}
	}
}

fn post_tick_system(input: &ItemStruct) -> TokenStream {
	let ident = &input.ident;

	let shared_fields = input.fields.iter().filter(is_shared);

	let prop_types = shared_fields
		.clone()
		.map(|field| {
			let ty = &field.ty;
			quote!(&mut #ty, )
		})
		.collect::<TokenStream>();

	let prop_destructs = shared_fields
		.clone()
		.map(|field| {
			let field_ident = &field.ident;
			quote!(mut #field_ident, )
		})
		.collect::<TokenStream>();

	let prop_assignments = shared_fields
		.map(|field| {
			let field_ident = &field.ident;
			quote!(*#field_ident = value.#field_ident;)
		})
		.collect::<TokenStream>();

	quote! {
		fn post_tick_system(&self) -> SystemConfigs {

			fn post_sync_system(mut query: Query<(&#ident,#prop_types), Changed<#ident>>){
				for (value, #prop_destructs) in query.iter_mut(){
					#prop_assignments
				}
			}

			post_sync_system.into_configs()
		}
	}
}


fn is_shared(field: &&syn::Field) -> bool {
	field
		.attrs
		.iter()
		.any(|attr| attr.meta.path().is_ident("shared"))
}

fn spawn(input: &ItemStruct) -> TokenStream {
	let insert_props = input
		.fields
		.iter()
		.filter(is_shared)
		.map(|field| {
			let field_name = &field.ident;
			// let field_type = &field.ty;
			quote! {
				entity.insert(self.#field_name.clone());
			}
		})
		.collect::<TokenStream>();


	quote! {
		fn spawn(&self, entity: &mut EntityWorldMut<'_>) {
			entity.insert(self.clone());
			#insert_props
		}
		fn spawn_with_command(&self, entity: &mut EntityCommands) {
			entity.insert(self.clone());
			#insert_props
		}
	}
}
