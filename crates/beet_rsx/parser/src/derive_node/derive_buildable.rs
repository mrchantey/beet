use crate::prelude::*;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::DeriveInput;
use syn::Result;

pub fn parse_derive_buildable(input: DeriveInput) -> TokenStream {
	parse(input).unwrap_or_else(|err| err.into_compile_error())
}

fn parse(input: DeriveInput) -> Result<TokenStream> {
	let fields = NodeField::parse_all(&input)?;
	let impl_buildable = impl_buildable(&input, &fields)?;
	let impl_flatten = impl_flatten(&input.ident, &input, &fields)?;
	let impl_self_as_ref_mut = impl_self_as_ref_mut(&input);
	// let impl_buildable_blanket = impl_buildable_blanket(&input);

	Ok(quote! {
		#impl_flatten
		#impl_buildable
		#impl_self_as_ref_mut
		// #impl_buildable_blanket
	})
}


fn impl_self_as_ref_mut(input: &DeriveInput) -> TokenStream {
	let target_ident = &input.ident;

	let (impl_generics, type_generics, where_clause) =
		input.generics.split_for_impl();

	quote! {
		impl #impl_generics AsRef<#target_ident #type_generics> for #target_ident #type_generics #where_clause {
			fn as_ref(&self) -> &Self {
				self
			}
		}
		impl #impl_generics AsMut<#target_ident #type_generics> for #target_ident #type_generics #where_clause {
			fn as_mut(&mut self) -> &mut Self {
				self
			}
		}
	}
}
fn impl_buildable(
	input: &DeriveInput,
	fields: &Vec<NodeField>,
) -> Result<TokenStream> {
	let field_methods = fields
		.iter()
		.map(|field| {
			let name = &field.ident;
			let actual_ty = &field.inner.ty;
			let (generics, builder_ty, expr) = NodeField::assign_tokens(field)?;
			let docs = field.docs();

			let expr = if field.is_optional() {
				quote! {Some(#expr)}
			} else {
				quote! {#expr}
			};

			let get = format_ident!("get_{}", name);
			let get_mut = format_ident!("get_{}_mut", name);
			let set = format_ident!("set_{}", name);

			Ok(quote! {
				#(#docs)*
				fn #name #generics(mut self, value: #builder_ty) -> Self {
					self.get_mut().#name = #expr;
					self
				}
				#(#docs)*
				fn #get(&self) -> & #actual_ty {
					&self.get().#name
				}
				#(#docs)*
				fn #get_mut(&mut self) -> &mut #actual_ty {
					&mut self.get_mut().#name
				}
				#(#docs)*
				fn #set #generics(&mut self, value: #builder_ty) -> &mut Self {
					self.get_mut().#name = #expr;
					self
				}
			})
		})
		.collect::<Result<Vec<_>>>()?;

	let target_ident = &input.ident;

	let (impl_generics, type_generics, where_clause) =
		input.generics.split_for_impl();

	let vis = &input.vis;
	let buildable_ident = name_lookup::buildable_ident(&input.ident);

	// let mut marker_impl_generics = input.generics.params.clone();
	// marker_impl_generics.push(syn::parse_quote! { M });
	let builder_docs = format!(
		" Builder for `{}`. This trait has a blanket implementation for any type that \
		implements AsRef and AsMut.",
		target_ident
	);

	// use BeetT to avoid collisions with other generics
	let blanket_impl_generics = extend_impl_generics(
		input,
		quote!(
			BeetT: AsRef<#target_ident #type_generics> + AsMut<#target_ident #type_generics>
		),
	);
	Ok(quote! {

		#[doc = #builder_docs]
		#vis trait #buildable_ident #impl_generics: Sized + #where_clause {
			fn get(&self) -> &#target_ident #type_generics;
			fn get_mut(&mut self) -> &mut #target_ident #type_generics;

			#(#field_methods)*
		}

		impl #blanket_impl_generics #buildable_ident #type_generics for BeetT #where_clause {
			fn get(&self) -> &#target_ident #type_generics { self.as_ref() }
			fn get_mut(&mut self) -> &mut #target_ident #type_generics { self.as_mut() }
		}
	})
}

#[allow(dead_code)]
// this didnt work, type cannot be inferred
fn impl_buildable_blanket(input: &DeriveInput) -> TokenStream {
	let buildable_ident = name_lookup::buildable_ident(&input.ident);
	let target_ident = &input.ident;

	let (_, type_generics, _) = input.generics.split_for_impl();


	let mut blanket_generics = input.generics.clone();
	blanket_generics
		.params
		.push(syn::parse_quote! { BuildableT });
	blanket_generics
		.params
		.push(syn::parse_quote! { BuildableMarker });
	blanket_generics.params.push(syn::parse_quote! { AsMutT });
	let buildable_type_generics =
		extend_type_generics(input, quote!(BuildableMarker));
	let marker_type_generics =
		extend_type_generics(input, quote!((BuildableT, BuildableMarker)));

	let (blanket_impl_generics, _type_generics, where_clause) =
		blanket_generics.split_for_impl();

	let mut where_clause = where_clause
		.cloned()
		.unwrap_or_else(|| syn::parse_quote!(where));
	where_clause.predicates.push(
		syn::parse_quote! {AsMutT: AsMut<BuildableT> + AsRef<BuildableT>},
	);
	where_clause.predicates.push(syn::parse_quote! {
		BuildableT: 'static + #buildable_ident #buildable_type_generics
	});

	quote! {
		// blanket impl for all types that implement AsMut<BuildableT>
		impl #blanket_impl_generics #buildable_ident #marker_type_generics for AsMutT #where_clause {
			fn get(&self) -> & #target_ident #type_generics { self.as_ref().get() }
			fn get_mut(&mut self) -> &mut #target_ident #type_generics { self.as_mut().get_mut() }
		}
	}
}


/// Appends the provided marker to the end of the impl generics,
/// ie `<T: ToString, U, V, Foo>`
fn extend_impl_generics(
	input: &DeriveInput,
	marker: TokenStream,
) -> TokenStream {
	let marker_impl_generics = input.generics.params.iter().collect::<Vec<_>>();
	if marker_impl_generics.is_empty() {
		return quote! {<#marker>};
	} else {
		quote! {<#(#marker_impl_generics),*, #marker>}
	}
}


/// Appends the provided marker to the end of the type generics,
/// ie `<T, U, V, Foo>`
fn extend_type_generics(
	input: &DeriveInput,
	marker: TokenStream,
) -> TokenStream {
	let marker_type_generics = input
		.generics
		.type_params()
		.map(|tp| &tp.ident)
		.collect::<Vec<_>>();
	if marker_type_generics.is_empty() {
		return quote! {<#marker>};
	} else {
		quote! {<#(#marker_type_generics),*, #marker>}
	}
}
