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



/// Add observers to a global action observer entity.
/// This macro must be placed above `#[derive(Component)]` as it
/// sets the `on_add` and `on_remove` hooks.
/// ## Example
/// ```rust ignore
/// #[action(log_on_run)]
/// #[derive(Component)]
/// struct LogOnRun(pub String);
///
/// fn log_on_run(trigger: Trigger<OnRun>, query: Populated<&LogOnRun>) {
/// 	let name = query.get(trigger.action).unwrap();
/// 	println!("log_name_on_run: {}", name.0);
/// }
/// ```
#[proc_macro_attribute]
pub fn action(
	attr: proc_macro::TokenStream,
	item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	impl_action_attr(attr, item)
}
