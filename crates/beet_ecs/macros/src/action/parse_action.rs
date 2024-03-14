use super::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;
use syn::Result;


pub fn parse_action(item: proc_macro::TokenStream) -> Result<TokenStream> {
	let input = syn::parse::<DeriveInput>(item)?;
	let args = ActionArgs::new(&input)?;

	let impl_register = parse_register(&args, &input);
	let impl_systems = parse_systems(&args, &input);
	let impl_child_components = parse_children(&args, &input);

	input.generics;

	Ok(quote! {
		use beet::prelude::*;
		use beet::exports::*;

		#impl_register
		#impl_systems
		#impl_child_components
	})
}

fn parse_register(args: &ActionArgs, input: &DeriveInput) -> TokenStream {
	let register_child_components = args
		.child_components
		.iter()
		.map(|c| {
			quote! {registry.register::<#c>();}
		})
		.collect::<TokenStream>();

	let ident = &input.ident;
	let (impl_generics, type_generics, where_clause) =
		&input.generics.split_for_impl();

	quote! {
		impl #impl_generics ActionTypes for #ident #type_generics #where_clause {
			fn register(registry: &mut TypeRegistry){
				#register_child_components
				registry.register::<#ident #type_generics>();
			}
		}
	}
}

fn parse_systems(args: &ActionArgs, input: &DeriveInput) -> TokenStream {
	if let Some(system) = &args.system {
		let set = &args.set;

		let ident = &input.ident;
		let (impl_generics, type_generics, where_clause) =
			&input.generics.split_for_impl();	

		quote! {
			impl #impl_generics ActionSystems for #ident #type_generics #where_clause {
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
	}
}

fn parse_children(args: &ActionArgs, input: &DeriveInput) -> TokenStream {
	if args.child_components.len() > 0 {
		let add_child_components = args
			.child_components
			.iter()
			.map(|c| {
				quote! {entity.insert(#c::default());}
			})
			.collect::<TokenStream>();
		let boxed_child_components = args
			.child_components
			.iter()
			.map(|c| {
				quote! {Box::new(#c::default()),}
			})
			.collect::<TokenStream>();

		let ident = &input.ident;

		let (impl_generics, type_generics, where_clause) =
			&input.generics.split_for_impl();


		quote! {
			impl #impl_generics ActionChildComponents for #ident #type_generics #where_clause {
				fn insert_child_components(&self, entity: &mut EntityWorldMut<'_>){
					#add_child_components
				}
				fn boxed_child_components(&self) -> Vec<Box<dyn Reflect>>{
					vec![
						#boxed_child_components
					]
				}
			}
		}
	} else {
		quote! {}
	}
}
