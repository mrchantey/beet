pub mod expect_action {
	use crate::prelude::*;
	use bevy::prelude::*;

	pub fn to_have_child<T: ActionPayload>(
		trig: &On<T>,
		child: Entity,
	) -> String {
		format!(
			"The child {:?} does not belong to the action {:?}",
			child, trig
		)
	}
	pub fn to_have_children<T: ActionPayload>(trig: &On<T>) -> String {
		format!("Action entity has no children: {:?}", trig)
	}

	/// Error for for when an [ActionContext] uses a placeholder
	/// and the request was made globally.
	pub fn to_have_action<T: ActionPayload>(trig: &On<T>) -> String {
		format!("Action entity is missing from query: {:?}", trig)
	}
	/// Error for for when an [ActionContext] uses a placeholder
	/// and the request was made globally.
	pub fn to_specify_action<T: ActionPayload>(trig: &On<T>) -> String {
		format!(
			"Globally triggered ActionContext must specify an action: {:?}",
			trig
		)
	}
}
