use crate::prelude::*;
use beet_common::prelude::*;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use syn::DeriveInput;
use syn::Expr;
use syn::ExprLit;
use syn::Ident;
use syn::Lit;
use syn::Result;
use syn::Type;


/// Impl AsRef<T> and AsMut<T> for all fields that are marked with #[field(flatten)]
/// Also impl AsRef<Type> and AsMut<Type> for all fields that are marked with
/// #[field(flatten=Type)] or #[field(flatten("Type<usize>"))]
/// ## `target_ident`
/// For `derive(Node)` this is NodeBuilder,
/// for `derive(Buildable)` this is input.ident
pub fn impl_flatten(
	target_ident: &Ident,
	input: &DeriveInput,
	fields: &Vec<NodeField>,
) -> Result<TokenStream> {
	let (impl_generics, type_generics, where_clause) =
		input.generics.split_for_impl();

	let flatten_impls = fields.iter()
			.filter(|field|field.attributes.contains("flatten"))
			.map(|field| {
				let field_ident = &field.ident;
				let field_type = &field.syn_field.ty;
				let second_order_as_mut = second_order_impl(target_ident,input, field)?;

				Ok(quote! {
					 impl #impl_generics AsRef<#field_type> for #target_ident #type_generics #where_clause {
						 fn as_ref(&self) -> &#field_type { &self.#field_ident }
						}
						impl #impl_generics AsMut<#field_type> for #target_ident #type_generics #where_clause {
							fn as_mut(&mut self) -> &mut #field_type { &mut self.#field_ident }
						}
						#(#second_order_as_mut)*
						// impl #impl_generics #field_buildable #marker_type_generics for #target_ident #type_generics #where_clause {
							// 	fn get(&self) -> &#field_type { &self.#field_name }
							// 	fn get_mut(&mut self) -> &mut #field_type { &mut self.#field_name }				
							// }
						})
					}
				).collect::<Result<Vec<_>>>()?;
	Ok(quote! {#(#flatten_impls)*})
}

// when A flattens B and B flattens C we use second order flattening
// to also implement the flattened builders
fn second_order_impl(
	target_ident: &Ident,
	input: &DeriveInput,
	field: &NamedField,
) -> Result<Vec<TokenStream>> {
	let (impl_generics, type_generics, where_clause) =
		input.generics.split_for_impl();
	let field_ident = &field.ident;

	let flatten_attrs = field.attributes.get_many("flatten");

	flatten_attrs.iter().filter_map(|attr|{
		attr.value.as_ref()
	}).map(|expr|{
		let ty: Type = if let Expr::Lit(ExprLit{lit,..}) = expr && let Lit::Str(lit) = &lit{
			syn::parse_str(&lit.value())?
		}else{
			syn::parse2(expr.to_token_stream())?
		};
		if ty == field.syn_field.ty {
			// its already implemented, should we warn unnecessary attr value?
			return Ok(TokenStream::default());
		}

		Ok(quote!{
			impl #impl_generics AsRef<#ty> for #target_ident #type_generics #where_clause {
				fn as_ref(&self) -> &#ty { self.#field_ident.as_ref() }
			}
			impl #impl_generics AsMut<#ty> for #target_ident #type_generics #where_clause {
				fn as_mut(&mut self) -> &mut #ty { self.#field_ident.as_mut() }
		 }
		})
	}).collect()
}
