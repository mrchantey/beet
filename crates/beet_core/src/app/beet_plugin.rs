use crate::prelude::*;
use beet_ecs::prelude::*;
use beet_net::prelude::*;
use bevy_app::prelude::*;
use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::prelude::*;
use bevy_time::TimePlugin;
use std::marker::PhantomData;

/// The plugin required for most beet apps
pub struct BeetMinimalPlugin;
impl Plugin for BeetMinimalPlugin {
	fn build(&self, app: &mut App) { app.add_plugins(TimePlugin); }
}


#[derive(Debug, Clone, Deref, DerefMut, Resource)]
pub struct RelayRes(pub Relay);

pub struct BeetPlugin<T: ActionPayload> {
	relay: Relay,
	_phantom: PhantomData<T>,
}
impl<T: ActionPayload> BeetPlugin<T> {
	pub fn new(relay: Relay) -> Self {
		Self {
			relay,
			_phantom: PhantomData,
		}
	}
}

impl<T: ActionPayload> Plugin for BeetPlugin<T> {
	fn build(&self, app: &mut App) {
		let mut relay = self.relay.clone();
		app.insert_resource(BeetEntityMap::default())
			.insert_resource(SpawnEntityHandler::<T>::new(&mut relay))
			.insert_resource(DespawnEntityHandler::new(&mut relay))
			.add_systems(
				PreUpdate,
				// despawn before spawn to avoid immediate despawn in case of despawn_all
				(
					handle_despawn_entity.pipe(log_error),
					handle_spawn_entity::<T>,
				)
					.chain(),
			)
			.add_plugins(ActionPlugin::<T, _>::default())
			.add_systems(
				PostUpdate,
				(send_position, cleanup_beet_entity_map.pipe(log_error)),
			)
			.insert_resource(RelayRes(relay));
	}
}

fn log_error<T>(result: In<anyhow::Result<T>>) {
	if let Err(e) = result.0 {
		log::error!("{}", e);
	}
}
