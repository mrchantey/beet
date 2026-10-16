use beet_core_shared::pkg_ext;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Data;
use syn::DeriveInput;
use syn::Fields;
use syn::parse_macro_input;

pub(crate) fn impl_string_primitive(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	parse(input)
		.unwrap_or_else(|err| err.into_compile_error())
		.into()
}

fn parse(input: DeriveInput) -> syn::Result<TokenStream> {
	let is_newtype = match &input.data {
		Data::Struct(data) => matches!(
			&data.fields,
			Fields::Unnamed(fields) if fields.unnamed.len() == 1
		),
		_ => false,
	};
	if !is_newtype {
		return Err(syn::Error::new_spanned(
			&input.ident,
			"`StringPrimitive` derives for a tuple struct over one `SmolStr`, \
			 ie `struct Did(SmolStr)`",
		));
	}
	let beet_core = pkg_ext::internal_or_beet("beet_core");
	let ident = &input.ident;
	let (impl_generics, type_generics, where_clause) =
		input.generics.split_for_impl();

	Ok(quote! {
		impl #impl_generics #ident #type_generics #where_clause {
			/// The text this value is written as.
			pub fn as_str(&self) -> &str { self.0.as_str() }
		}

		impl #impl_generics core::fmt::Display for #ident #type_generics #where_clause {
			fn fmt(
				&self,
				formatter: &mut core::fmt::Formatter,
			) -> core::fmt::Result {
				formatter.write_str(self.0.as_str())
			}
		}

		impl #impl_generics core::convert::AsRef<str> for #ident #type_generics #where_clause {
			fn as_ref(&self) -> &str { self.0.as_str() }
		}

		impl #impl_generics core::str::FromStr for #ident #type_generics #where_clause {
			type Err = #beet_core::prelude::BevyError;
			fn from_str(
				text: &str,
			) -> core::result::Result<Self, #beet_core::prelude::BevyError> {
				Self::parse(text)
			}
		}

		impl #impl_generics core::convert::TryFrom<#beet_core::prelude::SmolStr> for #ident #type_generics #where_clause {
			type Error = #beet_core::prelude::BevyError;
			fn try_from(
				text: #beet_core::prelude::SmolStr,
			) -> core::result::Result<Self, #beet_core::prelude::BevyError> {
				Self::parse(&text)
			}
		}

		impl #impl_generics core::convert::From<#ident #type_generics> for #beet_core::prelude::SmolStr #where_clause {
			fn from(value: #ident #type_generics) -> Self { value.0 }
		}
	})
}
