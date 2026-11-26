extern crate std;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use beet_core_shared::pkg_ext;
use heck::ToLowerCamelCase;
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;
use syn::LitStr;
use syn::parse_macro_input;

pub(crate) fn impl_def(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	parse(input)
		.unwrap_or_else(|err| err.into_compile_error())
		.into()
}

fn parse(input: DeriveInput) -> syn::Result<TokenStream> {
	let nsid = nsid(&input)?;
	let beet_core = pkg_ext::internal_or_beet("beet_core");
	let ident = &input.ident;
	let (impl_generics, type_generics, where_clause) =
		input.generics.split_for_impl();
	Ok(quote! {
		impl #impl_generics #beet_core::prelude::Def for #ident #type_generics #where_clause {
			const NSID: #beet_core::prelude::Nsid =
				#beet_core::prelude::Nsid::new_static(#nsid);
		}
	})
}

/// `#[def(nsid = "..")]` verbatim, else the lexicon (`#[def(lexicon = "..")]`
/// or the crate's own) and the type name camel cased.
fn nsid(input: &DeriveInput) -> syn::Result<String> {
	let mut nsid = None;
	let mut lexicon = None;
	for attr in input
		.attrs
		.iter()
		.filter(|attr| attr.path().is_ident("def"))
	{
		attr.parse_nested_meta(|meta| {
			let value = meta.value()?.parse::<LitStr>()?.value();
			if meta.path.is_ident("nsid") {
				nsid = Some(value);
			} else if meta.path.is_ident("lexicon") {
				lexicon = Some(value);
			} else {
				return Err(meta.error("expected `nsid` or `lexicon`"));
			}
			Ok(())
		})?;
	}
	if let Some(nsid) = nsid {
		return Ok(nsid);
	}
	let lexicon = match lexicon.or_else(crate_lexicon) {
		Some(lexicon) => lexicon,
		None => {
			return Err(syn::Error::new_spanned(
				&input.ident,
				"`Def` derives its lexicon from a `beet_*` crate's name; name \
				 this crate's with `#[def(lexicon = \"com.example.plugin\")]`",
			));
		}
	};
	Ok(format!(
		"{lexicon}#{}",
		input.ident.to_string().to_lower_camel_case()
	))
}

/// The lexicon a beet crate exports under: `beet_core` as `org.beet.core`,
/// `beet_foo_bar` as `org.beet.fooBar`.
fn crate_lexicon() -> Option<String> {
	std::env::var("CARGO_CRATE_NAME")
		.ok()?
		.strip_prefix("beet_")
		.map(|name| format!("org.beet.{}", name.to_lower_camel_case()))
}
