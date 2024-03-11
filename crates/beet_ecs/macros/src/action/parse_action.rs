use crate::utils::*;
use proc_macro2::TokenStream;
use quote::quote;
use quote::ToTokens;
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
	let input = syn::parse::<ItemStruct>(item)?;
	let args = &attributes_map(attr.into(), Some(&["system", "components","set"]))?;

	let action_trait = action_trait(&input, args);

	Ok(quote! {
		use beet::prelude::*;
		use beet::exports::*;
		// #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Component)]
		#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Component, FieldUi)]
		#input
		#action_trait
	})
}


fn action_trait(
	input: &ItemStruct,
	args: &HashMap<String, Option<Expr>>,
) -> TokenStream {
	let ident = &input.ident;

	let meta = meta(input);
	let tick_system = tick_system(args);

	let components = args
		.get("components")
		.map(|c| c.as_ref().map(|e| e.to_token_stream()))
		.flatten()
		.unwrap_or_default();

	let set = args
		.get("set")
		.map(|s| s.as_ref().map(|e| e.to_token_stream()))
		.flatten()
		.unwrap_or(quote! {TickSet});

	quote! {
		impl Action for #ident {
			fn duplicate(&self) -> Box<dyn Action> {
				Box::new(self.clone())
			}
			#meta
			fn insert_from_world(&self, entity: &mut EntityWorldMut<'_>){
				entity.insert((self.clone(),#components));
			}
			fn insert_from_commands(&self, entity: &mut EntityCommands){
				entity.insert((self.clone(),#components));
			}
		}

		impl ActionSystems for #ident{
			fn add_systems(app: &mut App, schedule: impl ScheduleLabel + Clone){
				#tick_system
				app.add_systems(
					schedule.clone(),
					tick_system.in_set(#set),
				);
			}
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
	// fn tick_system(&self) -> SystemConfigs {
		let tick_system = #expr;
	}
}
