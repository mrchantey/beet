use crate::prelude::*;
use bevy::prelude::*;

#[derive_action]
#[action(graph_role=GraphRole::World)]
/// Does what it says on the tin, useful for tests
pub struct EmptyAction;
pub fn empty_action() {}
