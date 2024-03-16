use proc_macro2::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::DeriveInput;
use syn::Expr;
use syn::Meta;
use syn::Result;
use syn::Token;

pub fn parse_action_list(item: proc_macro::TokenStream) -> Result<TokenStream> {
	let input = syn::parse::<DeriveInput>(item)?;
	let attrs = parse_attrs(&input)?;
	let ident = &input.ident;

	let impl_add_systems = attrs
		.iter()
		.map(|a| {
			quote! {
				#a::add_systems(app, schedule.clone());
			}
		})
		.collect::<TokenStream>();
	let impl_components = attrs
		.iter()
		.map(|a| {
			quote! {
				#a::register_components(world);
			}
		})
		.collect::<TokenStream>();
	let impl_types = attrs
		.iter()
		.map(|a| {
			quote! {
				#a::register_types(type_registry);
			}
		})
		.collect::<TokenStream>();

	let (impl_generics, type_generics, where_clause) =
		&input.generics.split_for_impl();

	Ok(quote! {
		use ::beet_ecs::prelude::*;
		use ::beet_ecs::exports::*;

		impl #impl_generics ActionSystems for #ident #type_generics #where_clause {
			fn add_systems(app: &mut App, schedule: impl ScheduleLabel + Clone) {
				#impl_add_systems
			}
		}

		impl #impl_generics ActionTypes for #ident #type_generics #where_clause {
				fn register_components(world: &mut World) {
					#impl_components
				}
				fn register_types(type_registry: &mut TypeRegistry) {
					#impl_types
				}
		}
	})
}

fn parse_attrs(input: &DeriveInput) -> Result<Vec<Expr>> {
	if let Some(attr) =
		input.attrs.iter().find(|a| a.path().is_ident("actions"))
	{
		match &attr.meta {
			Meta::List(list) => {
				let idents = list
					.parse_args_with(
						Punctuated::<Expr, Token![,]>::parse_terminated,
					)?
					.into_iter()
					.collect::<Vec<_>>();
				return Ok(idents);
			}
			_ => {}
		}
	}
	Err(syn::Error::new(
		input.ident.span(),
		"ActionList expected #[actions(MyAction1, MyAction2, ...)]",
	))
}
