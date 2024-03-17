mod action;
use action::*;
// mod field_ui;
// use field_ui::*;
use proc_macro::TokenStream;
mod utils;
// pub(crate) use utils::*;

#[proc_macro_derive(Action, attributes(action))]
pub fn action(item: TokenStream) -> TokenStream {
	parse_action(item)
		.unwrap_or_else(syn::Error::into_compile_error)
		.into()
}
#[proc_macro_derive(ActionList, attributes(actions))]
pub fn action_list(item: TokenStream) -> TokenStream {
	parse_action_list(item)
		.unwrap_or_else(syn::Error::into_compile_error)
		.into()
}



/// Minimal derives for an action, use to reduce boilerplate.
///
/// ```rust
///
/// #[derive_action(no_system)]
/// struct MyStruct{}
/// ```
///
/// is the same as this:
/// ```rust
/// #[derive(Debug, Clone, Component, Reflect, Action)]
///	#[reflect(Component)]
///	#[action(no_system)]
/// struct MyStruct{}
/// ```
///
#[proc_macro_attribute]
pub fn derive_action(attr: TokenStream, item: TokenStream) -> TokenStream {
	let item = syn::parse_macro_input!(item as syn::DeriveInput);
	let attr = proc_macro2::TokenStream::from(attr);
	quote::quote! {
		use ::beet::prelude::*;
		use ::beet::exports::*;
		#[derive(Debug, Clone, Component, Reflect, Action)]
		#[reflect(Component)]
		#[action(#attr)]
		#item
	}
	.into()
}



// #[proc_macro_attribute]
// #[proc_macro_derive(FieldUi, attributes(number, hide_ui))]
// pub fn field_ui(item: TokenStream) -> TokenStream {
// 	parse_field_ui(item)
// 		.unwrap_or_else(syn::Error::into_compile_error)
// 		.into()
// }
