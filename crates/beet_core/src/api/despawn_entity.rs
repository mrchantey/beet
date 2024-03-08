use crate::prelude::*;
use beet_net::prelude::*;
use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::prelude::*;
use forky_core::ResultTEExt;
use serde::Deserialize;
use serde::Serialize;

#[derive(Resource, Deref, DerefMut)]
pub struct DespawnEntityHandler(
	pub Responder<DespawnEntityPayload, Result<(), EntityNotFoundError>>,
);

impl DespawnEntityHandler {
	pub fn new(relay: &mut Relay) -> Self {
		Self(
			relay
				.add_responder(ENTITY_TOPIC, TopicMethod::Delete)
				.unwrap(), //should be correct topic
		)
	}

	pub fn requester(
		relay: &mut Relay,
	) -> Requester<DespawnEntityPayload, Result<(), EntityNotFoundError>> {
		relay
			.add_requester(ENTITY_TOPIC, TopicMethod::Delete)
			.unwrap() //should be correct topic
	}
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DespawnEntityPayload {
	/// If None, despawn all entities
	pub beet_id: Option<BeetEntityId>,
}

impl DespawnEntityPayload {
	pub fn all() -> Self { Self { beet_id: None } }
	pub fn new(beet_id: BeetEntityId) -> Self {
		Self {
			beet_id: Some(beet_id),
		}
	}
}

pub fn handle_despawn_entity(
	mut commands: Commands,
	entity_map: ResMut<BeetEntityMap>,
	mut handler: ResMut<DespawnEntityHandler>,
) {
	handler
		.try_handle_next(|val| {
			if let Some(beet_id) = val.beet_id {
				let entity = entity_map.get(beet_id)?;
				commands.entity(*entity).despawn();
			} else {
				for entity in entity_map.map().values() {
					commands.entity(*entity).despawn();
				}
			}
			Ok(())
		})
		.ok_or(|e| log::error!("{e}"));
}
