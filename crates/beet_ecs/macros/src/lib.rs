mod action;
use action::*;
mod beet_module;
use beet_module::*;
mod inspector_options;
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
#[proc_macro_derive(BeetModule, attributes(actions, components, bundles))]
pub fn beet_module(item: TokenStream) -> TokenStream {
	parse_beet_module(item)
		.unwrap_or_else(syn::Error::into_compile_error)
		.into()
}



/// Minimal derives for an action, use to reduce boilerplate.
#[proc_macro_attribute]
pub fn derive_action(attr: TokenStream, item: TokenStream) -> TokenStream {
	parse_derive_action(attr, item)
		.unwrap_or_else(syn::Error::into_compile_error)
		.into()
}

#[proc_macro_derive(InspectorOptions, attributes(inspector))]
pub fn inspectable(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	inspector_options::inspectable(input)
}



// #[proc_macro_attribute]
// #[proc_macro_derive(FieldUi, attributes(number, hide_ui))]
// pub fn field_ui(item: TokenStream) -> TokenStream {
// 	parse_field_ui(item)
// 		.unwrap_or_else(syn::Error::into_compile_error)
// 		.into()
// }
