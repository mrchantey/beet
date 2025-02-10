mod action;
mod utils;
use action::*;

/// Declare an action, this is a component that also
/// defines a relation between actions and the singleton observer.
///
/// This may be deprecated once we get many-many relations
#[proc_macro_derive(Action, attributes(observers, category, storage))]
pub fn derive_action(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	impl_derive_action(input)
}



/// Mark an observer type for use with the Action derive macro
#[proc_macro_attribute]
pub fn action(
	attr: proc_macro::TokenStream,
	item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	impl_action_attr(attr, item)
}
