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


pub const ENTITY_TOPIC: &'static str = "entity";

#[derive(Resource, Deref, DerefMut)]
pub struct SpawnEntityHandler<T: ActionPayload>(
	pub Responder<SpawnEntityPayload<T>, BeetEntityId>,
);

impl<T: ActionPayload> SpawnEntityHandler<T> {
	pub fn new(relay: &mut Relay) -> Self {
		Self(
			relay
				.add_responder(ENTITY_TOPIC, TopicMethod::Create)
				.unwrap(), //should be correct topic
		)
	}

	pub fn requester(
		relay: &mut Relay,
	) -> Requester<SpawnEntityPayload<T>, BeetEntityId> {
		relay
			.add_requester(ENTITY_TOPIC, TopicMethod::Create)
			.unwrap() //should be correct topic
	}
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnEntityPayload<T: ActionSuper> {
	pub position: Option<Vec3>,
	pub graph: Option<BehaviorGraph<T>>,
	pub position_tracking: bool,
}

impl<T: ActionSuper> Default for SpawnEntityPayload<T> {
	fn default() -> Self {
		Self {
			position: None,
			graph: None,
			position_tracking: false,
		}
	}
}



impl<T: ActionSuper> SpawnEntityPayload<T> {
	pub fn new(
		graph: Option<BehaviorGraph<T>>,
		position: Option<Vec3>,
		position_tracking: bool,
	) -> Self {
		Self {
			position,
			graph,
			position_tracking,
		}
	}
	pub fn with_position(mut self, position: Vec3) -> Self {
		self.position = Some(position);
		self
	}
	pub fn with_tracked_position(mut self, position: Vec3) -> Self {
		self.position_tracking = true;
		self.position = Some(position);
		self
	}
	pub fn with_graph(mut self, graph: impl Into<BehaviorGraph<T>>) -> Self {
		self.graph = Some(graph.into());
		self
	}
}

pub fn handle_spawn_entity<T: ActionPayload>(
	mut commands: Commands,
	mut entity_map: ResMut<BeetEntityMap>,
	mut handler: ResMut<SpawnEntityHandler<T>>,
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
			if let Some(graph) = val.graph {
				graph.spawn(&mut commands, entity);
			}

			beet_id
		})
		.ok_or(|e| log::error!("{e}"));
}
