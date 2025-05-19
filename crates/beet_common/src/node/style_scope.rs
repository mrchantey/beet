/// Define the scope of a style tag, set by using the `scope` template directive
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StyleScope {
	/// The default scope for a style tag, its styles will only be applied to
	/// elements within the component, each selector will be preprended with
	/// an attribute selector for the component, eg `[data-styleid-1]`.
	/// ## Example
	/// Remember `scope:local` is the default so this directive can be ommitted.
	/// ```rust ignore
	/// <style scope:local>
	/// 	div { color: blue; }
	/// </style>
	/// ```
	#[default]
	Local,
	/// Global scope for a style tag, its styles will not have an attribute
	/// selector prepended to them, so will apply to all elements in the document.
	/// The style tag will still be extracted and deduplicated.
	/// ## Example
	/// ```rust ignore
	/// <style scope:global>
	/// 	div { color: blue; }
	/// </style>
	/// ```
	Global,
}


impl StyleScope {}
#[cfg(feature = "tokens")]
impl crate::prelude::RustTokens for StyleScope {
	fn into_rust_tokens(&self) -> proc_macro2::TokenStream {
		match self {
			Self::Local => quote::quote! { StyleScope::Local },
			Self::Global => quote::quote! { StyleScope::Global },
		}
	}
}
