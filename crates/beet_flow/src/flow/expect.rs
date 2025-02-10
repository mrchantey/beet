pub mod expect_action {
	use bevy::prelude::*;
	use std::fmt::Debug;

	pub fn to_have_child(cx: impl Debug, child: Entity) -> String {
		format!(
			"The child {:?} does not belong to the action {:?}",
			child, cx
		)
	}
	pub fn to_have_children(cx: impl Debug) -> String {
		format!("Action entity has no children: {:?}", cx)
	}

	/// Error for for when an [ActionContext] uses a placeholder
	/// and the request was made globally.
	pub fn to_have_action(cx: impl Debug) -> String {
		format!("Action entity is missing from query: {:?}", cx)
	}
	/// Error for for when an [ActionContext] uses a placeholder
	/// and the request was made globally.
	pub fn to_specify_action(cx: impl Debug) -> String {
		format!(
			"Globally triggered ActionContext must specify an action: {:?}",
			cx
		)
	}
}
