use super::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemStruct;
use syn::Result;

// #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Component, FieldUi, Reflect)]

pub fn parse_action(item: proc_macro::TokenStream) -> Result<TokenStream> {
	let input = syn::parse::<ItemStruct>(item)?;
	let args = ActionArgs::new(&input)?;
	let ident = &input.ident;

	let impl_systems = if let Some(system) = args.system {
		let set = args.set;
		quote! {
			impl ActionSystems for #ident{
				fn add_systems(app: &mut App, schedule: impl ScheduleLabel + Clone){
					app.add_systems(
						schedule.clone(),
						#system.in_set(#set),
					);
				}
			}
		}
	} else {
		quote! {}
	};

	let impl_child_components = if args.child_components.len() > 0 {
		let add_child_components = args
			.child_components
			.iter()
			.map(|c| {
				quote! {entity.insert(#c::default());}
			})
			.collect::<TokenStream>();

		quote! {
			impl ActionChildComponents for #ident {
				fn insert_child_components(&self, entity: &mut EntityWorldMut<'_>){
					#add_child_components
				}
			}
		}
	} else {
		quote! {}
	};

	let register_child_components = args
		.child_components
		.iter()
		.map(|c| {
			quote! {registry.register::<#c>();}
		})
		.collect::<TokenStream>();

	Ok(quote! {
		use beet::prelude::*;
		use beet::exports::*;
		impl Action for #ident {
			fn duplicate(&self) -> Box<dyn Action> {
				Box::new(self.clone())
			}
			fn insert_from_world(&self, entity: &mut EntityWorldMut<'_>){
				entity.insert(self.clone());
			}
			fn insert_from_commands(&self, entity: &mut EntityCommands){
				entity.insert(self.clone());
			}
		}

		impl ActionTypes for #ident{
			fn register(registry: &mut TypeRegistry){
				#register_child_components
				registry.register::<#ident>();
			}
		}
		#impl_systems
		#impl_child_components
	})
}
