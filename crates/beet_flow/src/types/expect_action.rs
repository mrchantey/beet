//! Consistent error messages for action component queries.
//!
//! Actions frequently need to query for their associated components and related
//! entities. This module provides standardized error messages for when these
//! queries fail, improving debugging experience.
//!
//! Internal actions panic with these messages, but implementers may choose
//! to log the error and continue instead.
#[allow(unused, reason = "docs")]
use crate::prelude::*;
use beet_core::prelude::*;
use std::fmt::Debug;

/// Returns an error message when the action entity is missing from a query.
pub fn to_have_action(ev: impl Debug) -> String {
	format!("Action entity is missing from query: {:#?}", ev)
}

/// Returns an error message when the origin entity is missing from a query.
pub fn to_have_origin(ev: impl Debug) -> String {
	format!("Origin entity is missing from query: {:#?}", ev)
}

/// Returns an error when a specific child entity is not found.
///
/// This occurs when the child doesn't exist in the [`Children`] component
/// or the parent has no children at all.
pub fn to_have_child(ev: impl Debug, child: Entity) -> BevyError {
	bevyhow!(
		"The child {:?} does not belong to the action {:#?}",
		child,
		ev
	)
}

/// Returns an error when an action has no children.
///
/// This occurs when the action is missing a [`Children`] component
/// or the children list is empty.
pub fn to_have_children(ev: impl Debug) -> BevyError {
	bevyhow!("Action entity has no children: {:#?}", ev)
}

/// Returns an error message when an expected asset handle is not loaded.
pub fn to_have_asset(ev: impl Debug) -> String {
	format!("Action asset was not loaded: {:#?}", ev)
}

/// Returns an error message when an arbitrary entity is missing from a query.
pub fn to_have_other(ev: impl Debug) -> String {
	format!("Other entity is missing from query: {:#?}", ev)
}
