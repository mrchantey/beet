use crate::prelude::*;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::DeriveInput;
use syn::Result;

pub fn impl_buildable(input: DeriveInput) -> TokenStream {
	parse(input).unwrap_or_else(|err| err.into_compile_error())
}

fn parse(input: DeriveInput) -> Result<TokenStream> {
	let fields = PropsField::parse_all(&input)?;
	let set_val_methods = fields.iter().map(|field| {
		let name = &field.inner.ident;
		let (ty, expr) = field.assign_tokens();
		let docs = field.docs();

		let expr = if field.is_optional() {
			quote! {Some(#expr)}
		} else {
			quote! {#expr}
		};

		quote! {
			#(#docs)*
			fn #name(mut self, value: #ty) -> Self {
				self.as_mut().#name = #expr;
				self
			}
		}
	});

	let target_name = &input.ident;
	let trait_buildable_name = format_ident!("{}Buildable", &input.ident);

	let (impl_generics, type_generics, where_clause) =
		input.generics.split_for_impl();
	let vis = &input.vis;

	let as_mut = fields.iter()
		.filter(|field|field.attributes.contains("flatten"))
		.map(|field| {
			let field_name = &field.inner.ident;
			let field_type = &field.inner.ty;
			Some(quote! {
			   impl #impl_generics AsMut<#field_type> for #target_name #type_generics #where_clause {
				   fn as_mut(&mut self) -> &mut #field_type { &mut self.#field_name }
			   }
			})
		}
	);


	let mut blanket_impl_generics = input.generics.params.clone();
	blanket_impl_generics.push(syn::parse_quote! { T });

	Ok(quote! {
		#[allow(missing_docs)]
		#vis trait #trait_buildable_name #impl_generics: Sized + AsMut<#target_name #type_generics> #where_clause {
			#(#set_val_methods)*
		}

		impl <#blanket_impl_generics> #trait_buildable_name for T where T: AsMut<#target_name #type_generics> #where_clause {
		}

		impl #impl_generics AsMut<Self> for #target_name #type_generics #where_clause {
			fn as_mut(&mut self) -> &mut Self { self }
		}

		#(#as_mut)*

	})
}
