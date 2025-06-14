use crate::utils::pound_token;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn;
use syn::DeriveInput;
use syn::parse_macro_input;

pub fn impl_sendit(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	parse(input)
		.unwrap_or_else(|err| err.into_compile_error())
		.into()
}

fn parse(input: DeriveInput) -> syn::Result<TokenStream> {
	let (impl_generics, type_generics, where_clause) =
		input.generics.split_for_impl();
	let vis = &input.vis;
	let input_ident = &input.ident;
	let ident = format_ident!("{}Send", input.ident);

	// Extract sendit attributes and convert them
	let sendit_attrs: Vec<TokenStream> = input
		.attrs
		.iter()
		.filter_map(|attr| {
			if attr.path().is_ident("sendit") {
				let pound_token = pound_token();
				let attrs = attr.parse_args::<TokenStream>().unwrap();
				// let tokens = &list.tokens;
				Some(quote!(#pound_token [#attrs]))
			} else {
				None
			}
		})
		.collect();

	Ok(quote! {
		#(#sendit_attrs)*
		#vis struct #ident <#impl_generics>(send_wrapper::SendWrapper<#input_ident #type_generics>) #where_clause;

		impl #impl_generics #ident #type_generics #where_clause{
			pub fn new(value: #input_ident #type_generics) -> Self {
				Self(send_wrapper::SendWrapper::new(value))
			}
			pub fn inner(self) -> #input_ident #type_generics {
				self.0.take()
			}
		}
		impl std::ops::Deref for #ident <#impl_generics> #where_clause {
			type Target = #input_ident #type_generics;
			fn deref(&self) -> &Self::Target {
				&self.0
			}
		}
		impl std::ops::DerefMut for #ident <#impl_generics> #where_clause {
			fn deref_mut(&mut self) -> &mut Self::Target {
				&mut self.0
			}
		}
	})
}



#[cfg(test)]
mod test {
	use super::*;
	use beet_utils::prelude::*;
	use quote::quote;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let input: DeriveInput = syn::parse_quote! {
			#[sendit(derive(Clone))]
			pub struct ElementNode {
				pub self_closing: bool,
			}
		};

		input.xmap(parse).unwrap().to_string().xpect().to_be(
			quote! {
				#[derive(Clone)]
				pub struct ElementNodeSend<>(send_wrapper::SendWrapper<ElementNode>);
				impl ElementNodeSend {
					pub fn new(value: ElementNode) -> Self {
						Self(send_wrapper::SendWrapper::new(value))
					}
					pub fn inner(self) -> ElementNode {
						self.0.take()
					}
				}
				impl std::ops::Deref for ElementNodeSend<> {
					type Target = ElementNode;
					fn deref(&self) -> &Self::Target {
						&self.0
					}
				}
				impl std::ops::DerefMut for ElementNodeSend<> {
					fn deref_mut(&mut self) -> &mut Self::Target {
						&mut self.0
					}
				}
			}
			.to_string(),
		);
	}
}
