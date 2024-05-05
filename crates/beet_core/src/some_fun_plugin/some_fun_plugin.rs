use super::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;
use std::borrow::Cow;

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
pub struct SomeFunPlugin;

impl Plugin for SomeFunPlugin {
	fn build(&self, app: &mut App) {
		app /*-*/
			// .add_systems(PreUpdate, auto_spawn.before(PreTickSet))
			.add_systems(Update, randomize_position.in_set(PreTickSet))
		/*-*/;

		let world = app.world_mut();

		world.init_component::<AutoSpawn>();
		world.init_component::<RandomizePosition>();
		world.init_component::<RenderText>();

		let mut registry =
			world.get_resource::<AppTypeRegistry>().unwrap().write();
		registry.register::<AutoSpawn>();
		registry.register::<RandomizePosition>();
		registry.register::<RenderText>();
	}
}
