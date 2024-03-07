use crate::prelude::*;
use beet_net::prelude::*;
use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::prelude::*;
use bevy_math::prelude::*;
use bevy_transform::components::Transform;
use bevy_transform::TransformBundle;
use bevy_utils::default;
use forky_core::ResultTEExt;
use serde::Deserialize;
use serde::Serialize;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct SpawnEntityPayload {
	pub position: Option<Vec3>,
	pub position_tracking: bool,
}
impl SpawnEntityPayload {
	pub fn with_position(self, pos: Vec3) -> Self {
		SpawnEntityPayload {
			position: Some(pos),
			..self
		}
	}
	pub fn with_position_tracking(self) -> Self {
		SpawnEntityPayload {
			position_tracking: true,
			..self
		}
	}
}

#[derive(Resource, Deref, DerefMut)]
pub struct SpawnEntityHandler(pub Responder<SpawnEntityPayload, BeetEntityId>);


impl SpawnEntityHandler {
	pub const ADDRESS: &'static str = "entity";
	pub const METHOD: TopicMethod = TopicMethod::Create;
	pub fn new(relay: &mut Relay) -> Self {
		Self(
			relay
				.add_responder(Self::ADDRESS, TopicMethod::Create)
				.unwrap(), //should be correct topic
		)
	}

	pub fn requester(
		relay: &mut Relay,
	) -> Requester<SpawnEntityPayload, BeetEntityId> {
		relay
			.add_requester(Self::ADDRESS, TopicMethod::Create)
			.unwrap() //should be correct topic
	}
}


pub fn handle_spawn_entity(
	mut commands: Commands,
	mut entity_map: ResMut<BeetEntityMap>,
	mut handler: ResMut<SpawnEntityHandler>,
) {
	handler
		.try_handle_next(|val| {
			let mut entity = commands.spawn_empty();
			let beet_id = entity_map.next(entity.id());
			entity.insert(beet_id);
			if val.position_tracking {
				entity.insert(TrackedPosition);
			}
			if let Some(pos) = val.position {
				entity.insert(TransformBundle {
					local: Transform::from_translation(pos),
					..default()
				});
			}
			beet_id
		})
		.ok_or(|e| log::error!("{e}"));
}
