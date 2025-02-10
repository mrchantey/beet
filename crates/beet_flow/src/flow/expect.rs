pub mod expect_action {
	// use crate::prelude::*;
	use bevy::prelude::*;

	/// Error for for when an [ActionContext] uses a placeholder
	/// and the request was made globally.
	pub fn to_have_action(action: Entity) -> String {
		format!("Action entity is missing from query: {:?}", action)
	}
	/// Error for for when an [ActionContext] uses a placeholder
	/// and the request was made globally.
	pub fn to_specify_action(action: Entity) -> String {
		format!("Globally triggered ActionContext must specify an action, received: {}",action)
	}
}
