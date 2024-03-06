use super::*;
use beet_net::prelude::*;
use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::prelude::*;
use bevy_math::prelude::*;
use bevy_transform::components::Transform;
use forky_core::ResultTEExt;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentPosition {
	pub entity_id: BeetEntityId,
	pub pos: Vec3,
}

#[derive(Resource, Deref, DerefMut)]
pub struct PositionSender(pub Publisher<SentPosition>);

impl PositionSender {
	pub fn topic() -> Topic { Topic::pubsub_update("entity/position") }
	pub fn new(relay: &mut Relay) -> Self {
		Self(relay.add_publisher_with_topic(Self::topic()).unwrap())
	}
}


#[derive(Component)]
pub struct TrackedPosition;

pub fn send_position(
	sender: ResMut<PositionSender>,
	query: Query<
		(&BeetEntityId, &Transform),
		(With<TrackedPosition>, Changed<Transform>),
	>,
) {
	for (id, transform) in query.iter() {
		sender
			.send(&SentPosition {
				entity_id: *id,
				pos: transform.translation,
			})
			.ok_or(|e| log::error!("{e}"));
	}
}
