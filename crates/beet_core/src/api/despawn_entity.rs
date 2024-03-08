use crate::prelude::*;
use anyhow::Result;
use beet_net::prelude::*;
use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::prelude::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(Resource, Deref, DerefMut)]
pub struct DespawnEntityHandler(pub Subscriber<DespawnEntityPayload>);

impl DespawnEntityHandler {
	pub fn new(relay: &mut Relay) -> Self { Self(Self::subscriber(relay)) }
	pub fn subscriber(relay: &mut Relay) -> Subscriber<DespawnEntityPayload> {
		relay
			.add_subscriber(ENTITY_TOPIC, TopicMethod::Delete)
			.unwrap() //should be correct topic
	}

	pub fn publisher(relay: &mut Relay) -> Publisher<DespawnEntityPayload> {
		relay
			.add_publisher(ENTITY_TOPIC, TopicMethod::Delete)
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
) -> Result<()> {
	for msg in handler.try_recv_all()? {
		if let Some(beet_id) = msg.beet_id {
			let entity = entity_map.get(beet_id)?;
			commands.entity(*entity).despawn();
		} else {
			for entity in entity_map.map().values() {
				commands.entity(*entity).despawn();
			}
		}
	}

	Ok(())
}
