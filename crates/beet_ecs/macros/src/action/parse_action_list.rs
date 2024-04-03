use proc_macro2::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::Attribute;
use syn::DeriveInput;
use syn::Expr;
use syn::Meta;
use syn::Result;
use syn::Token;

pub fn parse_action_list(item: proc_macro::TokenStream) -> Result<TokenStream> {
	let input = syn::parse::<DeriveInput>(item)?;
	let actions = parse_actions_attr(&input)?;
	let components = parse_components_attr(&input)?;
	let ident = &input.ident;

	let impl_add_systems = actions
		.iter()
		.map(|a| {
			quote! {
				#a::add_systems(app, schedule.clone());
			}
		})
		.collect::<TokenStream>();

	let impl_components = actions
		.iter()
		.map(|a| {
			quote! {
				#a::register_components(world);
			}
		})
		.collect::<TokenStream>();

	let impl_components2 = components
		.iter()
		.map(|a| {
			quote! {
				world.init_component::<#a>();
			}
		})
		.collect::<TokenStream>();

	let impl_types = actions
		.iter()
		.map(|a| {
			quote! {
				#a::register_types(type_registry);
			}
		})
		.collect::<TokenStream>();

	let impl_types2 = components
		.iter()
		.map(|a| {
			quote! {
				type_registry.register::<#a>();
			}
		})
		.collect::<TokenStream>();

	let (impl_generics, type_generics, where_clause) =
		&input.generics.split_for_impl();

	Ok(quote! {
		use ::beet::prelude::*;
		use ::beet::exports::*;

		impl #impl_generics ActionSystems for #ident #type_generics #where_clause {
			fn add_systems(app: &mut App, schedule: impl ScheduleLabel + Clone) {
				#impl_add_systems
			}
		}

		impl #impl_generics ActionTypes for #ident #type_generics #where_clause {
				fn register_components(world: &mut World) {
					#impl_components
					#impl_components2
				}
				fn register_types(type_registry: &mut TypeRegistry) {
					#impl_types
					#impl_types2
				}
		}
	})
}


fn parse_actions_attr(input: &DeriveInput) -> Result<Vec<Expr>> {
	if let Some(attr) =
		input.attrs.iter().find(|a| a.path().is_ident("actions"))
	{
		parse_list_attr(attr)
	} else {
		Err(syn::Error::new(
			input.ident.span(),
			"ActionList expected #[actions(MyAction1, MyAction2, ...)]",
		))
	}
}
fn parse_components_attr(input: &DeriveInput) -> Result<Vec<Expr>> {
	if let Some(attr) =
		input.attrs.iter().find(|a| a.path().is_ident("components"))
	{
		parse_list_attr(attr)
	} else {
		Ok(vec![])
	}
}

fn parse_list_attr(attr: &Attribute) -> Result<Vec<Expr>> {
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
		_ => Err(syn::Error::new(
			attr.span(),
			"Expected a list: #[some_attr(Foo,Bar,Baz)]",
		)),
	}
}
