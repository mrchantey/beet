/// Define the scope of a style tag, set by using the `scope` template directive
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StyleScope {
	/// The default scope for a style tag, its styles will only be applied to
	/// elements within the component, each selector will be preprended with
	/// an attribute selector for the component, eg `[data-styleid-1]`.
	/// ## Example
	/// ```rust ignore
	/// <style scope:local>
	/// 	div { color: blue; }
	/// </style>
	/// ```
	#[default]
	Local,
	/// Global scope for a style tag, its styles will not have an attribute
	/// selector prepended to them, so will apply to all elements in the document.
	/// ## Example
	/// ```rust ignore
	/// <style scope:global>
	/// 	div { color: blue; }
	/// </style>
	/// ```
	Global,
	/// This style tag should not be extracted at all, beet
	/// will leave it as is.
	Verbatim,
}


impl StyleScope {}
#[cfg(feature = "tokens")]
impl crate::prelude::SerdeTokens for StyleScope {
	fn into_rust_tokens(&self) -> proc_macro2::TokenStream {
		match self {
			Self::Local => quote::quote! { StyleScope::Local },
			Self::Global => quote::quote! { StyleScope::Global },
			Self::Verbatim => quote::quote! { StyleScope::Verbatim },
		}
	}

	fn into_ron_tokens(&self) -> proc_macro2::TokenStream {
		match self {
			Self::Local => quote::quote! { Local },
			Self::Global => quote::quote! { Global },
			Self::Verbatim => quote::quote! { Verbatim },
		}
	}
}
