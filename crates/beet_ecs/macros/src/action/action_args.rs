use crate::utils::*;
use proc_macro2::TokenStream;
use quote::quote;
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::Attribute;
use syn::Expr;
use syn::Ident;
use syn::ItemStruct;
use syn::Result;

pub struct ActionArgs {
	pub set: TokenStream,
	pub system: Option<TokenStream>,
	pub child_components: Vec<Ident>,
}

impl ActionArgs {
	pub fn new(input: &ItemStruct) -> Result<Self> {
		let args = Self::get_args(&input.attrs)?;

		let snake_case = heck::AsSnakeCase(input.ident.to_string());
		let snake_case =
			Ident::new(&snake_case.to_string(), input.span()).to_token_stream();
		let mut set = quote! {TickSet};
		let mut system = Some(snake_case);
		let mut child_components = Vec::new();

		if args.paths.contains_key("no_system") {
			system = None;
		}
		if let Some(new_system) = args.exprs.get("system") {
			system = Some(new_system.to_token_stream());
		}
		if let Some(new_set) = args.exprs.get("set") {
			set = new_set.to_token_stream();
		}
		if let Some(new_child_components) = args.exprs.get("child_components") {
			child_components = match new_child_components {
				Expr::Array(expr_array) => {
					let mut components = Vec::new();
					for expr in &expr_array.elems {
						if let Expr::Path(expr_path) = expr {
							if let Some(ident) = expr_path.path.get_ident() {
								components.push(ident.clone());
							}
						}
					}
					components
				}
				_ => {
					return Err(syn::Error::new(
						new_child_components.span(),
						"Expected an array of components, ie `child_components = [Comp1, Comp2]`",
					));
				}
			};
		}
		Ok(Self {
			system,
			set,
			child_components,
		})
	}

	fn get_args(attrs: &Vec<Attribute>) -> Result<AttributesMap> {
		if let Some(attr) =
			attrs.iter().find(|a| a.meta.path().is_ident("action"))
		{
			let args: TokenStream = attr.parse_args().unwrap_or_default();
			let args = AttributesMap::new(args, &["no_system"], &[
				"system",
				"child_components",
				"set",
			])?;
			Ok(args)
		} else {
			Ok(AttributesMap::default())
		}
	}
}
