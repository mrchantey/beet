mod action;
use action::*;
mod field_ui;
use field_ui::*;
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
/// #[action(always_pass)]
/// struct AlwaysPass{
/// 	#[shared]
///   score: Score,
/// }
///
///
/// fn always_pass(entities: Query<&mut AlwaysPass, With<Running>>) {
///
/// 	for mut node in entities.iter_mut() {
///   	node.score = Score::Pass;
/// 	}
/// }
///
/// ```
///
/// It also adds a syncing system
/// TODO use Added, Removed instead of Option<AlwaysPass>
/// ```rust
/// fn sync_always_pass(mut query: Query<(Option<&mut Score>, Option<AlwaysPass>), With<AlwaysPass>>) {
///
///  // if they are equal, use commands or mutate etc.
///
/// }
/// ```
///
/// ## Derives
///
/// [`Debug`], [`Clone`], [`PartialEq`], [`serde::Serialize`], [`serde::Deserialize`], [`bevy::Component`]
///
///
#[proc_macro_attribute]
pub fn action(attr: TokenStream, item: TokenStream) -> TokenStream {
	parse_action(attr, item)
		.unwrap_or_else(syn::Error::into_compile_error)
		.into()
}



// #[proc_macro_attribute]
#[proc_macro_derive(FieldUi, attributes(number, hide_ui))]
pub fn field_ui(item: TokenStream) -> TokenStream {
	parse_field_ui(item)
		.unwrap_or_else(syn::Error::into_compile_error)
		.into()
}
