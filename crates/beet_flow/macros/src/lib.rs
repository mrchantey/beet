mod action;
mod utils;


///
/// ## Attributes
/// `observers`
/// Observers that are spawned when this component is added and despawned when it is removed.
///
/// `systems`
/// Systems for long running behaviors
///
/// ```rust
///	# use bevy::prelude::*;
///	# use beet_flow::prelude::*;
///
/// #[derive(Component, Action)]
/// #[observers(log_name_on_run)]
/// struct LogOnRun(pub String);
///
/// fn log_name_on_run(trigger: Trigger<OnRun>, query: Query<&LogOnRun>) {
/// 	let name = query
/// 		.get(trigger.entity())
/// 		.map(|n| n.0.as_str())
/// 		.unwrap();
/// 	println!("log_name_on_run: {name}");
/// }
///
///
/// ```
#[proc_macro_derive(
	Action,
	attributes(observers, global_observers, systems, category, storage)
)]
pub fn action(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	action::derive_action(input)
}

/// An action is a superset of a [`Component`]
#[proc_macro_derive(GlobalAction, attributes(observers, category, storage))]
pub fn global_action(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	action::derive_action_global(input)
}
