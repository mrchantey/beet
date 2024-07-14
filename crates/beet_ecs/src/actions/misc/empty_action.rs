use crate::prelude::*;
use bevy::prelude::*;

/// Does what it says on the tin, useful for tests etc
#[derive(Debug, Default, Clone, PartialEq, Component, Action, Reflect)]
#[reflect(Default, Component, ActionMeta)]
#[category(ActionCategory::Behavior)]
pub struct EmptyAction;
