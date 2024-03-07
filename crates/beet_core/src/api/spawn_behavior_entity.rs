use crate::prelude::*;
use beet_ecs::prelude::*;
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

#[derive(Resource, Deref, DerefMut)]
pub struct SpawnBehaviorEntityHandler<T: ActionPayload>(
	pub Responder<SpawnBehaviorEntityPayload<T>, BeetEntityId>,
);

impl<T: ActionPayload> SpawnBehaviorEntityHandler<T> {
	pub const ADDRESS: &'static str = "behavior_entity";
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
	) -> Requester<SpawnBehaviorEntityPayload<T>, BeetEntityId> {
		relay
			.add_requester(Self::ADDRESS, TopicMethod::Create)
			.unwrap() //should be correct topic
	}
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnBehaviorEntityPayload<T: ActionSuper> {
	pub position: Option<Vec3>,
	pub graph: BehaviorGraph<T>,
	pub position_tracking: bool,
}
impl<T: ActionSuper> SpawnBehaviorEntityPayload<T> {
	pub fn new(
		graph: BehaviorGraph<T>,
		position: Option<Vec3>,
		position_tracking: bool,
	) -> Self {
		Self {
			position,
			graph,
			position_tracking,
		}
	}
}

pub fn handle_spawn_behavior_entity<T: ActionPayload>(
	mut commands: Commands,
	mut entity_map: ResMut<BeetEntityMap>,
	mut handler: ResMut<SpawnBehaviorEntityHandler<T>>,
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
			let entity = entity.id();

			let graph = val.graph.spawn(&mut commands, entity);
			commands.entity(entity).insert(graph);

			beet_id
		})
		.ok_or(|e| log::error!("{e}"));
}
