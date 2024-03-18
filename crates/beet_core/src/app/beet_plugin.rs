use crate::prelude::*;
use beet_ecs::prelude::*;
use beet_net::prelude::*;
use bevy::app::PluginGroupBuilder;
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

impl<T: ActionList> Default for BeetPlugin<T> {
	fn default() -> Self {
		Self {
			relay: default(),
			_phantom: default(),
		}
	}
}

impl<T: ActionList> BeetPlugin<T> {
	pub fn new(relay: Relay) -> Self {
		Self {
			relay,
			_phantom: PhantomData,
		}
	}
}

pub struct DefaultBeetPlugins<T: ActionList> {
	pub beet: BeetPlugin<T>,
	pub steering: SteeringPlugin,
	pub bevy_event: BevyEventPlugin,
}

impl<T: ActionList> DefaultBeetPlugins<T> {
	pub fn new(type_registry: AppTypeRegistry) -> Self {
		Self {
			beet: BeetPlugin::new(Relay::new(0)),
			steering: SteeringPlugin::default(),
			bevy_event: BevyEventPlugin::new(type_registry),
		}
	}
}

impl PluginGroup for DefaultBeetPlugins<CoreNode> {
	fn build(self) -> PluginGroupBuilder {
		PluginGroupBuilder::start::<Self>()
			.add(self.beet)
			.add(self.steering)
			.add(self.bevy_event)
	}
}


impl<T: ActionList> Plugin for BeetPlugin<T> {
	fn build(&self, app: &mut App) {
		T::register_components(&mut app.world);
		T::register_types(&mut app.world.resource::<AppTypeRegistry>().write());
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
