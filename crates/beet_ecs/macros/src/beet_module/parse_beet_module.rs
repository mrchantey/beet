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

pub fn parse_beet_module(item: proc_macro::TokenStream) -> Result<TokenStream> {
	let input = syn::parse::<DeriveInput>(item)?;
	let actions = parse_named_list_attr(&input, "actions")?;
	let bundles = parse_named_list_attr(&input, "bundles")?;
	let components = parse_named_list_attr(&input, "components")?;
	let add_systems = add_systems(&actions);
	let register_types = register_types(&actions, &components);
	let register_bundles = register_bundles(&actions, &bundles, &components);


	let ident = &input.ident;
	let (impl_generics, type_generics, where_clause) =
		&input.generics.split_for_impl();

	Ok(quote! {
		use ::beet::prelude::*;
		use ::beet::exports::*;

		impl #impl_generics ActionSystems for #ident #type_generics #where_clause {
			#add_systems
		}

		impl #impl_generics BeetModule for #ident #type_generics #where_clause {
			#register_bundles
			#register_types
		}
	})
}

fn add_systems(actions: &Vec<Expr>) -> TokenStream {
	let impl_add_systems = actions
		.iter()
		.map(|a| {
			quote! {
				#a::add_systems(app, schedule.clone());
			}
		})
		.collect::<TokenStream>();

	quote! {
		fn add_systems(app: &mut App, schedule: impl ScheduleLabel + Clone) {
			#impl_add_systems
		}
	}
}

fn register_types(actions: &Vec<Expr>, components: &Vec<Expr>) -> TokenStream {
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

	quote! {
		fn register_types(type_registry: &mut TypeRegistry) {
			#impl_types
			#impl_types2
		}
	}
}


fn register_bundles(
	actions: &Vec<Expr>,
	bundles: &Vec<Expr>,
	components: &Vec<Expr>,
) -> TokenStream {
	let register_bundles_actions = actions
		.iter()
		.map(|a| {
			quote! {
				#a::register_bundles(world);
			}
		})
		.collect::<TokenStream>();

	let register_bundles_components = components
		.iter()
		.map(|a| {
			quote! {
				world.init_component::<#a>();
			}
		})
		.collect::<TokenStream>();
	let register_bundles_bundles = bundles
		.iter()
		.map(|a| {
			quote! {
				world.init_bundle::<#a>();
			}
		})
		.collect::<TokenStream>();

	quote! {
		fn register_bundles(world: &mut World) {
			#register_bundles_components
			#register_bundles_bundles
			#register_bundles_actions
		}
	}
}

fn parse_named_list_attr(input: &DeriveInput, name: &str) -> Result<Vec<Expr>> {
	input
		.attrs
		.iter()
		.find(|a| a.path().is_ident(name))
		.map(|attr| parse_list_attr(attr))
		.unwrap_or(Ok(Vec::new()))
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
