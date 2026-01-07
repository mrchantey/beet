use proc_macro2::Span;
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
	let ident = format_ident!("{}Sendit", input.ident);

	// Extract sendit attributes and convert them
	let sendit_attrs: Vec<TokenStream> = input
		.attrs
		.iter()
		.filter_map(|attr| {
			if attr.path().is_ident("sendit") {
				let pound_token = syn::Token![#](Span::call_site());
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
		#vis struct #ident #impl_generics(beet::exports::SendWrapper<#input_ident #type_generics>) #where_clause;

		impl #impl_generics #ident #type_generics #where_clause{
			pub fn new(value: #input_ident #type_generics) -> Self {
				Self(beet::exports::SendWrapper::new(value))
			}
			pub fn inner(self) -> #input_ident #type_generics {
				self.0.take()
			}
		}
		impl #impl_generics std::ops::Deref for #ident #type_generics #where_clause {
			type Target = #input_ident #type_generics;
			fn deref(&self) -> &Self::Target {
				&self.0
			}
		}
		impl #impl_generics std::ops::DerefMut for #ident #type_generics #where_clause {
			fn deref_mut(&mut self) -> &mut Self::Target {
				&mut self.0
			}
		}

		impl #impl_generics #input_ident #type_generics #where_clause {
			pub fn sendit(self) -> #ident #type_generics {
				#ident::new(self)
			}
		}
	})
}



#[cfg(test)]
mod test {
	use super::*;
	use quote::quote;
	use beet_core::prelude::*;

	#[test]
	fn works() {
		let input: DeriveInput = syn::parse_quote! {
					#[sendit(derive(Clone))]
		pub struct Foo<T: ToString>
		where
			T: std::fmt::Display
		{
			inner: T
		}
		};

		parse(input).unwrap().to_string().xpect_eq(
			quote! {
				#[derive(Clone)]
				pub struct FooSendit<T: ToString>(beet::exports::SendWrapper< Foo<T> >) where T: std::fmt::Display;

				impl<T: ToString> FooSendit<T> where T: std::fmt::Display {
					pub fn new(value: Foo<T>) -> Self {
						Self(beet::exports::SendWrapper::new(value))
					}
					pub fn inner(self) -> Foo<T> {
						self.0.take()
					}
				}
				impl<T: ToString> std::ops::Deref for FooSendit<T> where T: std::fmt::Display {
					type Target = Foo<T>;
					fn deref(&self) -> &Self::Target {
						&self.0
					}
				}
				impl<T: ToString> std::ops::DerefMut for FooSendit<T> where T: std::fmt::Display {
					fn deref_mut(&mut self) -> &mut Self::Target {
						&mut self.0
					}
				}

				impl<T: ToString> Foo<T> where T: std::fmt::Display {
					pub fn sendit(self) -> FooSendit<T> {
						FooSendit::new(self)
					}
				}
			}
			.to_string(),
		);
	}
}
