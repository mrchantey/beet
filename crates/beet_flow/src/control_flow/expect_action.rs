//! Actions frequently need to query for their associated
//! components and other entities. This module provides
//! consistent error messsages for when these queries fail.
//! Internal actions panic with these messages, but implementers may choose
//! to simply log the error and continue.
#[allow(unused, reason = "docs")]
use crate::prelude::*;
use bevy::prelude::*;
use std::fmt::Debug;

/// This event is missing a child entity, either because
/// it doesnt have a [`Children`] component or the child
/// was not found in the [`Children`] component.
pub fn to_have_child(ev: impl Debug, child: Entity) -> String {
	format!(
		"The child {:?} does not belong to the action {:#?}",
		child, ev
	)
}
/// This event is missing a [`Children`] component,
/// or it is empty.
pub fn to_have_children(ev: impl Debug) -> String {
	format!("Action entity has no children: {:#?}", ev)
}

/// The origin, ie [`OnRun::origin`] could not be found.
pub fn to_have_origin(ev: impl Debug) -> String {
	format!("Origin entity is missing from query: {:#?}", ev)
}
/// The action, ie [`OnRun::action`] could not be found.
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
