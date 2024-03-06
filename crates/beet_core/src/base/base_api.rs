use beet_ecs::graph::ActionSuper;
use beet_ecs::graph::BehaviorGraph;
use beet_net::pubsub::Payload;
use beet_net::pubsub::Requester;
use beet_net::pubsub::Responder;
use beet_net::relay::Relay;
use beet_net::topic::TopicMethod;
use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::prelude::*;
use bevy_math::prelude::*;
use bevy_transform::components::Transform;
use bevy_transform::TransformBundle;
use bevy_utils::default;
use bevy_utils::HashMap;
use forky_core::ResultTEExt;
use serde::Deserialize;
use serde::Serialize;


pub trait ActionPayload: Payload + ActionSuper {}
impl<T: Payload + ActionSuper> ActionPayload for T {}


pub type BeetEntityId = u64;
/// Incrementable without `ResMut` beause uses AtomicUsize
#[derive(Default, Resource)]
pub struct BeetEntityMap {
	id_incr: BeetEntityId,
	map: HashMap<BeetEntityId, Entity>,
}

impl BeetEntityMap {
	pub fn next(&mut self, entity: Entity) -> BeetEntityId {
		let next_id = self.id_incr.wrapping_add(1);
		self.id_incr = next_id;
		self.map.insert(next_id, entity);
		next_id
	}
}

pub struct SpawnedEntityMap(pub HashMap<u64, Entity>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnEntity {
	pub pos: Option<Vec3>,
}
impl SpawnEntity {
	pub fn with_position(pos: Vec3) -> Self { Self { pos: Some(pos) } }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnBehaviorEntity<T: ActionSuper> {
	pub pos: Option<Vec3>,
	pub graph: BehaviorGraph<T>,
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

#[derive(Resource, Deref, DerefMut)]
pub struct SpawnBehaviorEntityHandler<T: ActionPayload>(
	pub Responder<SpawnBehaviorEntity<T>, BeetEntityId>,
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
	) -> Requester<SpawnBehaviorEntity<T>, BeetEntityId> {
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
	// handler
	// 	.try_handle_next_blocking(|val| {
	// 		let mut entity = commands.spawn_empty();
	// 		if let Some(pos) = val.pos {
	// 			entity.insert(TransformBundle {
	// 				local: Transform::from_translation(pos),
	// 				..default()
	// 			});
	// 		}
	// 		entity_map.next(entity.id())
	// 	})
	// 	.ok_or(|e| log::error!("{e}"));
}
pub fn handle_spawn_behavior_entity<T: ActionPayload>(
	mut commands: Commands,
	mut entity_map: ResMut<BeetEntityMap>,
	mut handler: ResMut<SpawnBehaviorEntityHandler<T>>,
) {
	// handler
	// 	.try_handle_next(|val| {
	// 		let mut entity = commands.spawn_empty();

	// 		if let Some(pos) = val.pos {
	// 			entity.insert(TransformBundle {
	// 				local: Transform::from_translation(pos),
	// 				..default()
	// 			});
	// 		}
	// 		let entity = entity.id();

	// 		let graph = val.graph.spawn(&mut commands, entity);
	// 		commands.entity(entity).insert(graph);

	// 		entity_map.next(entity)
	// 	})
	// .ok_or(|e| log::error!("{e}"));
}
