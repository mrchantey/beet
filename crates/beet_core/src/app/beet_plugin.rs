use crate::prelude::*;
use beet_ecs::prelude::*;
use beet_net::prelude::*;
use bevy::prelude::*;
use bevy::time::TimePlugin;
use std::marker::PhantomData;

/// The plugin required for most beet apps
pub struct BeetMinimalPlugin;
impl Plugin for BeetMinimalPlugin {
	fn build(&self, app: &mut App) { app.add_plugins(TimePlugin); }
}


#[derive(Debug, Clone, Deref, DerefMut, Resource)]
pub struct RelayRes(pub Relay);

pub struct BeetPlugin<T: ActionList> {
	relay: Relay,
	_phantom: PhantomData<T>,
}
impl<T: ActionList> BeetPlugin<T> {
	pub fn new(relay: Relay) -> Self {
		Self {
			relay,
			_phantom: PhantomData,
		}
	}
}

impl<T: ActionList> Plugin for BeetPlugin<T> {
	fn build(&self, app: &mut App) {
		T::register_components(&mut app.world);
		let mut relay = self.relay.clone();
		app.insert_resource(BeetEntityMap::default())
			.insert_resource(TypedBehaviorPrefab::<T>::type_registry())
			.insert_resource(SpawnEntityHandler::<T>::new(&mut relay).unwrap())
			.insert_resource(DespawnEntityHandler::new(&mut relay).unwrap())
			.add_systems(
				PreUpdate,
				// despawn before spawn to avoid immediate despawn in case of despawn_all
				(
					handle_despawn_entity.pipe(log_error),
					handle_spawn_entity::<T>.pipe(log_error),
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
		// eprintln!("{}", e);
		log::error!("{}", e);
	}
}
