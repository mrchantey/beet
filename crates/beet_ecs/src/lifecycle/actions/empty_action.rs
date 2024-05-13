use crate::prelude::*;
use bevy::ecs::schedule::IntoSystemConfigs;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;

/// Does what it says on the tin, useful for tests etc
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component, ActionMeta)]
pub struct EmptyAction;

impl ActionMeta for EmptyAction {
	fn category(&self) -> ActionCategory { ActionCategory::World }
}

impl ActionSystems for EmptyAction {
	fn systems() -> SystemConfigs { (|| {}).into_configs() }
}
