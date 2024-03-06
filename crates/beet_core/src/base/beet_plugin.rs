use crate::prelude::*;
use beet_ecs::prelude::*;
use beet_net::prelude::*;
use bevy_app::prelude::*;
use std::marker::PhantomData;


pub struct BeetPlugin<T: ActionSuper + Payload> {
	relay: Relay,
	_phantom: PhantomData<T>,
}
impl<T: ActionSuper + Payload> BeetPlugin<T> {
	pub fn new(relay: Relay) -> Self {
		Self {
			relay,
			_phantom: PhantomData,
		}
	}
}

impl<T: ActionSuper + Payload> Plugin for BeetPlugin<T> {
	fn build(&self, app: &mut App) {
		let mut relay = self.relay.clone();
		app.insert_resource(SpawnEntityHandler::new(&mut relay))
			.insert_resource(SpawnBehaviorEntityHandler::<T>::new(&mut relay))
			.insert_resource(BeetEntityMap::default());

		app.add_systems(
			PreUpdate,
			(handle_spawn_entity, handle_spawn_behavior_entity::<T>),
		);
	}
}
