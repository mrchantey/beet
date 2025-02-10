pub mod expect_action {
	use bevy::prelude::*;
	use std::fmt::Debug;

	pub fn to_have_child(ev: impl Debug, child: Entity) -> String {
		format!(
			"The child {:?} does not belong to the action {:#?}",
			child, ev
		)
	}
	pub fn to_have_children(ev: impl Debug) -> String {
		format!("Action entity has no children: {:#?}", ev)
	}

	/// Error for for when an [ActionContext] uses a placeholder
	/// and the request was made globally.
	pub fn to_have_action(ev: impl Debug) -> String {
		format!("Action entity is missing from query: {:#?}", ev)
	}
	/// Error for for when an [ActionContext] uses a placeholder
	/// and the request was made globally.
	pub fn to_specify_action(ev: impl Debug) -> String {
		format!(
			"Globally triggered ActionContext must specify an action: {:#?}",
			ev
		)
	}
}
