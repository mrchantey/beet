extern crate alloc;

use alloc::vec::Vec;
use beet_core_shared::prelude::*;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::DeriveInput;
use syn::parse_macro_input;

pub fn impl_from_tokens(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	parse(input)
		.unwrap_or_else(|err| err.into_compile_error())
		.into()
}

fn parse(input: DeriveInput) -> syn::Result<TokenStream> {
	let ident = &input.ident;
	let vis = &input.vis;
	let tokens_ident = format_ident!("{}Tokens", ident);
	let (impl_generics, type_generics, where_clause) =
		input.generics.split_for_impl();

	// struct-level non-derive attrs (docs, cfg, etc.)
	let struct_attrs = &input.attrs;

	let syn::Data::Struct(data) = &input.data else {
		synbail!(ident, "FromTokens only supports structs");
	};
	let syn::Fields::Named(fields) = &data.fields else {
		synbail!(ident, "FromTokens only supports named fields");
	};

	let mut tokens_fields: Vec<TokenStream> = Vec::new();
	let mut from_tokens_bindings: Vec<TokenStream> = Vec::new();
	let mut field_names: Vec<_> = Vec::new();

	for field in &fields.named {
		let f_ident = &field.ident;
		let f_vis = &field.vis;
		let f_ty = &field.ty;
		let has_token = field.attrs.iter().any(|a| a.path().is_ident("token"));

		// strip `#[token]` from forwarded attrs
		let field_attrs: Vec<_> = field
			.attrs
			.iter()
			.filter(|a| !a.path().is_ident("token"))
			.collect();

		if has_token {
			tokens_fields.push(quote! {
				#(#field_attrs)*
				#f_vis #f_ident: Token,
			});
			from_tokens_bindings.push(quote! {
				let #f_ident = document_query.get_token(entity, &tokens.#f_ident)?.into_reflect()?;
			});
		} else {
			tokens_fields.push(quote! {
				#(#field_attrs)*
				#f_vis #f_ident: #f_ty,
			});
			from_tokens_bindings.push(quote! {
				let #f_ident = tokens.#f_ident;
			});
		}

		field_names.push(f_ident);
	}

	Ok(quote! {
		#(#struct_attrs)*
		#[derive(Debug, Clone, PartialEq, Reflect)]
		#vis struct #tokens_ident #impl_generics #where_clause {
			#(#tokens_fields)*
		}

		impl #impl_generics FromTokens for #ident #type_generics #where_clause {
			type Tokens = #tokens_ident #type_generics;
			fn from_tokens(
				tokens: Self::Tokens,
				entity: Entity,
				document_query: &DocumentQuery,
			) -> Result<Self> {
				#(#from_tokens_bindings)*
				Ok(Self { #(#field_names),* })
			}
		}
	})
}

#[cfg(test)]
mod test {
	use super::parse;
	use alloc::string::ToString;
	use syn::DeriveInput;

	#[test]
	fn generates_tokens_struct_and_impl() {
		let input: DeriveInput = syn::parse_quote! {
			/// A motion token.
			#[derive(Debug, Clone, PartialEq, Reflect, FromTokens)]
			pub struct Motion {
				#[token]
				pub duration: Duration,
				pub ease: EaseFunction,
			}
		};
		let result = parse(input).unwrap().to_string();
		// Tokens struct should be present
		assert!(
			result.contains("MotionTokens"),
			"expected MotionTokens in output"
		);
		// FromTokens impl should be present
		assert!(
			result.contains("FromTokens for Motion"),
			"expected impl FromTokens for Motion"
		);
		// token field replaced with Token type
		assert!(
			result.contains("duration : Token"),
			"expected duration: Token"
		);
		// plain field unchanged
		assert!(
			result.contains("ease : EaseFunction"),
			"expected ease: EaseFunction"
		);
		// get_token call for duration
		assert!(result.contains("get_token"), "expected get_token call");
		// plain passthrough for ease
		assert!(
			result.contains("let ease = tokens . ease"),
			"expected ease passthrough"
		);
		// FromTokens not in derive list for the Tokens struct
		assert!(
			!result.contains("derive (FromTokens"),
			"FromTokens should not be in Tokens struct derives"
		);
	}
}
