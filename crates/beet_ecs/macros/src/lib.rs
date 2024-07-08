mod action;
mod utils;



///
///
///
/// ## Attributes
/// `observers`
/// Observers that are spawned when this component is added and despawned when it is removed.
#[proc_macro_derive(Action, attributes(observers,observers_non_generic))]
pub fn action(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	action::derive_action(input)
}
