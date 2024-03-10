use crate::prelude::*;
use beet_ecs::prelude::*;
use beet_net::prelude::*;
use bevy_core::Name;
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

// these fields are all hacks until BevyReflect, Scene serialization etc
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnEntityPayload<T: ActionSuper> {
	pub name: String,
	pub position: Option<Vec3>,
	pub graph: Option<BehaviorGraph<T>>,
	pub position_tracking: bool,
}

impl<T: ActionSuper> Default for SpawnEntityPayload<T> {
	fn default() -> Self {
		Self {
			name: "New Entity".to_string(),
			position: None,
			graph: None,
			position_tracking: false,
		}
	}
}



impl<T: ActionSuper> SpawnEntityPayload<T> {
	pub fn new(
		name: String,
		graph: Option<BehaviorGraph<T>>,
		position: Option<Vec3>,
		position_tracking: bool,
	) -> Self {
		Self {
			name,
			position,
			graph,
			position_tracking,
		}
	}
	pub fn with_name(mut self, name: impl Into<String>) -> Self {
		self.name = name.into();
		self
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
// This is a hack until BevyReflect, Scene serialization etc
pub fn handle_spawn_entity<T: ActionPayload>(
	mut commands: Commands,
	mut entity_map: ResMut<BeetEntityMap>,
	mut handler: ResMut<SpawnEntityHandler<T>>,
) {
	handler
		.try_handle_next(|val| {
			let SpawnEntityPayload {
				name,
				position,
				graph,
				position_tracking,
			} = val;

			let mut entity = commands.spawn(Name::new(name));
			let beet_id = entity_map.next(entity.id());
			entity.insert(beet_id);

			if position_tracking {
				entity.insert(TrackedPosition);
			}

			if let Some(pos) = position {
				entity.insert(TransformBundle {
					local: Transform::from_translation(pos),
					..default()
				});
			}
			if let Some(graph) = graph {
				entity.insert((
					ForceBundle::default(),
					SteerBundle {
						wander_params: WanderParams {
							outer_distance: 0.2,
							outer_radius: 0.1,
							inner_radius: 0.02,
							last_local_target: default(),
						},
						max_force: MaxForce(0.1),
						max_speed: MaxSpeed(0.3),
						..default()
					}
					.with_target(Vec3::ZERO),
				));
				let id = entity.id();
				graph.spawn(&mut commands, id);
			}

			beet_id
		})
		.ok_or(|e| log::error!("{e}"));
}
