use crate::prelude::*;
use beet_core::prelude::*;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::DeriveInput;
use syn::Ident;
use syn::Result;

/// Generates props implementation for a struct.
///
/// Creates the necessary trait implementations for using the struct
/// as props in RSX templates.
pub fn parse_derive_props(input: DeriveInput) -> TokenStream {
	parse(input).unwrap_or_else(|err| err.into_compile_error())
}

fn parse(input: DeriveInput) -> Result<TokenStream> {
	let fields = NodeField::parse_derive_input(&input)?;
	let impl_props = impl_props(&input)?;
	let impl_builder = impl_builder(&input, &fields)?;
	let impl_required = impl_required(&input, &fields)?;

	let beet_dom = pkg_ext::internal_or_beet("beet_dom");

	Ok(quote! {
		use #beet_dom::prelude::*;

		#impl_props
		#impl_builder
		#impl_required
	})
}

fn impl_props(input: &DeriveInput) -> Result<TokenStream> {
	let name = &input.ident;
	let builder_ident = name_lookup::builder_ident(&input.ident);
	let required_ident = name_lookup::required_ident(&input.ident);

	let (impl_generics, type_generics, where_clause) =
		input.generics.split_for_impl();

	Ok(quote! {
		impl #impl_generics Props for #name #type_generics #where_clause {
			type Builder = #builder_ident #type_generics;
			type Required = #required_ident;
		}
	})
}


fn impl_builder(
	input: &DeriveInput,
	fields: &[NodeField],
) -> Result<TokenStream> {
	let builder_fields = fields.iter().map(|field| {
		let name = &field.ident;
		let ty = field.inner_ty;
		let attrs = field.attrs;
		if field.is_default() {
			quote! {
				#(#attrs)*
				#name: #ty
			}
		} else {
			quote! {
				#(#attrs)*
				#name: Option<#ty>
			}
		}
	});

	let default_fallback = syn::parse_quote! { Default::default() };

	let builder_defaults = fields.iter().map(|field| {
		let name = &field.ident;
		if let Some(attr) = field.field_attributes.get("default") {
			let val = attr.value.as_ref().unwrap_or(&default_fallback);
			quote! { #name: #val }
		} else {
			quote! { #name: Default::default() }
		}
	});

	let unwrap_fields = fields.iter().map(|field| {
		let name = &field.ident;

		let rhs = if field.is_default() {
			quote! { self.#name }
		} else if field.is_optional() {
			quote! { self.#name }
		} else {
			let err_msg = format!(
				"Missing required field `{}::{}`",
				input.ident, field.ident
			);
			quote! { self.#name.expect(#err_msg) }
		};
		quote! {#name: #rhs}
	});


	let node_name = &input.ident;
	let builder_ident = name_lookup::builder_ident(&input.ident);
	let (impl_generics, type_generics, where_clause) =
		input.generics.split_for_impl();
	let vis = &input.vis;

	Ok(quote! {
		#[allow(missing_docs)]
		#[derive(Buildable)]
		#vis struct #builder_ident #impl_generics {
			#(#builder_fields),*
		}

		// impl #impl_generics #builder_ident #type_generics #where_clause {
		// 	#(#set_val_methods)*
		// }

		impl #impl_generics Default for #builder_ident #type_generics #where_clause {
			fn default() -> Self {
				Self {
					#(#builder_defaults),*
				}
			}
		}

		impl #impl_generics PropsBuilder for #builder_ident #type_generics #where_clause {
			type Props = #node_name #type_generics;

			fn build(self) -> Self::Props {
				Self::Props{
					#(#unwrap_fields),*
				}
			}
		}
	})
}

fn impl_required(
	input: &DeriveInput,
	fields: &[NodeField],
) -> Result<TokenStream> {
	let required_field_names = fields.iter().filter_map(|field| {
		if field.is_required() {
			Some(&field.ident)
		} else {
			None
		}
	});

	let impl_required_name = format_ident!("{}Required", &input.ident);
	let vis = &input.vis;

	Ok(quote! {
		#[allow(missing_docs)]
		#[derive(Default)]
		#vis struct #impl_required_name {
			#(pub #required_field_names: bool),*
		}
	})
}

mod name_lookup {
	use super::*;
	pub fn builder_ident(ident: &Ident) -> Ident {
		format_ident!("{}Builder", ident)
	}
	pub fn required_ident(ident: &Ident) -> Ident {
		format_ident!("{}Required", ident)
	}
}
#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn works() {
		parse_derive_props(syn::parse_quote! {
			#[derive(Node)]
			#[node(into_rsx = my_node)]
			struct MyNode {
				is_required: u32,
				is_optional: Option<u32>,
				#[field(default = 7)]
				is_default: u32,
				#[field(default)]
				is_generic_default: Foo<u32>,
			}
		})
		.xpect_snapshot();
	}
}
