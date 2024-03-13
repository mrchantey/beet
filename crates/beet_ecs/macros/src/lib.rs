mod action;
use action::*;
// mod field_ui;
// use field_ui::*;
use proc_macro::TokenStream;
mod utils;
// pub(crate) use utils::*;




/// Used for selectors aka non-leaf nodes.
/// Define props required for each child. Children should only be added to this
/// node if they have all the required props.
#[proc_macro_attribute]
pub fn child_props(_attr: TokenStream, _item: TokenStream) -> TokenStream {
	todo!()
}

/// Annotate a struct as an action, defining its corresponding system.
///
/// An action struct treats each field as a [`Prop`] which is a supertrait of [`Component, Clone`]
/// and only one of each type is allowed. This pattern allows all node systems to be run in parallel
/// and their props to be efficiently synced later.
///
/// ```rust
///
/// // a similar thing can be done for `AlwaysSuccceed`
///
/// #[action(system=always_pass,bundle=Score)]
/// struct AlwaysPass;
///
///
/// fn always_pass(entities: Query<&mut Score, (With<Running>, With<AlwaysPass>)>) {
///
/// 	for mut score in entities.iter_mut() {
///   	score = Score::Pass;
/// 	}
/// }
///
/// ```
///
///
///
#[proc_macro_derive(Action, attributes(action))]
pub fn action(item: TokenStream) -> TokenStream {
	parse_action(item)
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
	let item = syn::parse_macro_input!(item as syn::ItemStruct);
	let attr = proc_macro2::TokenStream::from(attr);
	quote::quote! {
		use beet::prelude::*;
		use beet::exports::*;
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
