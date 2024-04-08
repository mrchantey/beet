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
	let modules = parse_named_list_attr(&input, "modules")?;
	let actions = parse_named_list_attr(&input, "actions")?;
	let components = parse_named_list_attr(&input, "components")?;
	let actions_and_components =
		actions.iter().chain(components.iter()).collect::<Vec<_>>();
	let bundles = parse_named_list_attr(&input, "bundles")?;
	let add_systems = add_systems(&modules, &actions);
	let register_types = register_types(&modules, &actions_and_components);
	let register_bundles =
		register_bundles(&modules, &actions_and_components, &bundles);
	let ids = ids(&modules, &actions, &components, &bundles);


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
			#ids
		}
	})
}

fn add_systems(modules: &Vec<Expr>, actions: &Vec<Expr>) -> TokenStream {
	let modules = modules
		.iter()
		.map(|m| {
			quote! {
				#m::add_systems(app, schedule.clone());
			}
		})
		.collect::<TokenStream>();
	let actions = actions
		.iter()
		.map(|a| {
			quote! {
				#a::add_systems(app, schedule.clone());
			}
		})
		.collect::<TokenStream>();

	quote! {
		fn add_systems(app: &mut App, schedule: impl ScheduleLabel + Clone) {
			#modules
			#actions
		}
	}
}

fn register_types(
	modules: &Vec<Expr>,
	actions_and_components: &Vec<&Expr>,
) -> TokenStream {
	let modules = modules
		.iter()
		.map(|m| {
			quote! {
				#m::register_types(type_registry);
			}
		})
		.collect::<TokenStream>();
	let actions_and_components = actions_and_components
		.iter()
		.map(|c| {
			quote! {
				type_registry.register::<#c>();
			}
		})
		.collect::<TokenStream>();

	quote! {
		fn register_types(type_registry: &mut TypeRegistry) {
			#modules
			#actions_and_components
		}
	}
}


fn register_bundles(
	modules: &Vec<Expr>,
	actions_and_components: &Vec<&Expr>,
	bundles: &Vec<Expr>,
) -> TokenStream {
	let modules = modules
		.iter()
		.map(|m| {
			quote! {
				#m::register_bundles(world);
			}
		})
		.collect::<TokenStream>();
	let actions_and_components = actions_and_components
		.iter()
		.map(|c| {
			quote! {
				world.init_component::<#c>();
			}
		})
		.collect::<TokenStream>();
	let bundles = bundles
		.iter()
		.map(|b| {
			quote! {
				world.init_bundle::<#b>();
			}
		})
		.collect::<TokenStream>();

	quote! {
		fn register_bundles(world: &mut World) {
			#modules
			#actions_and_components
			#bundles
		}
	}
}

fn ids(
	modules: &Vec<Expr>,
	actions: &Vec<Expr>,
	components: &Vec<Expr>,
	bundles: &Vec<Expr>,
) -> TokenStream {
	let modules = modules
		.iter()
		.map(|m| {
			quote! {
				ids.extend(#m::infos());
			}
		})
		.collect::<TokenStream>();

	let actions = into_ids(actions, quote! {BeetType::Action});
	let components = into_ids(components, quote! {BeetType::Component});
	let bundles = into_ids(bundles, quote! {BeetType::Bundle});

	quote! {
		fn infos()->Vec<BeetTypeInfo> {
			let mut ids = vec![#actions #components #bundles];
			#modules
			ids
		}
	}
}

fn into_ids(items: &Vec<Expr>, ty: TokenStream) -> TokenStream {
	items
		.iter()
		.map(|i| {
			quote! {
				BeetTypeInfo::new::<#i>(#ty),
			}
		})
		.collect::<TokenStream>()
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
