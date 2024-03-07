use super::*;
use beet_net::prelude::*;
use bevy_ecs::prelude::*;
use bevy_transform::components::Transform;
use forky_core::ResultTEExt;

pub struct PositionSender;

impl PositionSender {
	pub fn topic(id: BeetEntityId) -> Topic {
		Topic::pubsub_update(format!("entity:{id}/position"))
	}
}

#[derive(Component)]
pub struct TrackedPosition;

pub fn send_position(
	relay: ResMut<RelayRes>,
	query: Query<
		(&BeetEntityId, &Transform),
		(With<TrackedPosition>, Changed<Transform>),
	>,
) {
	for (id, transform) in query.iter() {
		let topic = PositionSender::topic(*id);
		relay
			.send(topic, &transform.translation)
			.ok_or(|e| log::error!("{e}"));
	}
}
