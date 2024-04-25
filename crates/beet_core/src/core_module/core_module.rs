use super::*;
use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;
use std::borrow::Cow;

#[derive(Debug, Default, Clone, BeetModule)]
#[modules(EcsModule, MovementModule, SteerModule, RoboticsModule)]
#[components(AutoSpawn, RandomizePosition, RenderText)]
/// Collection of all built-in modules.
pub struct CoreModule;

#[derive(Component, Deref, DerefMut, Reflect)]
#[reflect(Component, Default)]
pub struct RenderText(pub Cow<'static, str>);

impl RenderText {
	pub fn new(text: impl Into<Cow<'static, str>>) -> Self { Self(text.into()) }
}

impl Default for RenderText {
	fn default() -> Self { Self::new("🥕") }
}

#[derive(Default)]
pub struct CorePlugin;

impl Plugin for CorePlugin {
	fn build(&self, app: &mut App) {
		app /*-*/
			.add_systems(PreUpdate, auto_spawn.before(PreTickSet))
			.add_systems(Update, randomize_position.in_set(PreTickSet))
		/*-*/;
	}
}
