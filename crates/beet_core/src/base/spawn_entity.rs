use super::*;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnEntity {
	pub pos: Option<Vec3>,
}
impl SpawnEntity {
	pub fn with_position(pos: Vec3) -> Self { Self { pos: Some(pos) } }
}

#[derive(Resource, Deref, DerefMut)]
pub struct SpawnEntityHandler(pub Responder<SpawnEntity, BeetEntityId>);


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
	) -> Requester<SpawnEntity, BeetEntityId> {
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
			if let Some(pos) = val.pos {
				entity.insert(TransformBundle {
					local: Transform::from_translation(pos),
					..default()
				});
			}
			beet_id
		})
		.ok_or(|e| log::error!("{e}"));
}
