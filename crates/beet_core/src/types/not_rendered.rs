//! [`NotRendered`]: an entity no render walks.
use crate::prelude::*;

/// An entity that is behaviour rather than content, so no render walks it or
/// its subtree, ie a script beet runs ([`RunScript`] requires it).
///
/// A render walk reads it as one `Without<NotRendered>` through a query that
/// keeps Bevy's default filters, so a held [`Prefab`] subtree and an authored
/// [`Disabled`] entity are left out by those filters rather than by name.
/// Never registered, so never saved: whatever requires it adds it again on
/// load.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Component)]
pub struct NotRendered;
