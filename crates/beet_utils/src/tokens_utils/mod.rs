pub mod pkg_ext;


/// workaround for the `#` token which cannot be escaped in a quote! macro
pub fn pound_token() -> syn::Token![#] {
	syn::Token![#](proc_macro2::Span::call_site())
}

