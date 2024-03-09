use crate::prelude::*;
use bevy_core::Name;
use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::prelude::*;
use bevy_utils::prelude::default;
use bevy_utils::HashMap;
use petgraph::graph::DiGraph;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Deref, DerefMut, Serialize, Deserialize, Component)]
pub struct EntityGraph(pub DiGraph<Entity, ()>);


pub struct EntityGraphOptions {
	target: Option<Entity>,
	run_on_spawn: bool,
}

impl Default for EntityGraphOptions {
	fn default() -> Self {
		Self {
			target: None,
			run_on_spawn: true,
		}
	}
}


impl EntityGraph {
	pub fn new<T: ActionSuper>(
		world: &mut impl WorldOrCommands,
		graph: impl Into<BehaviorGraph<T>>,
		target: Entity,
	) -> Self {
		Self::new_with_options(world, graph, EntityGraphOptions {
			target: Some(target),
			..default()
		})
	}
	pub fn new_no_target<T: ActionSuper>(
		world: &mut impl WorldOrCommands,
		graph: impl Into<BehaviorGraph<T>>,
	) -> Self {
		Self::new_with_options(world, graph, EntityGraphOptions {
			target: None,
			..default()
		})
	}

	pub fn new_with_options<T: ActionSuper>(
		world: &mut impl WorldOrCommands,
		graph: impl Into<BehaviorGraph<T>>,
		options: EntityGraphOptions,
	) -> Self {
		let graph = graph.into();
		let EntityGraphOptions {
			target,
			run_on_spawn,
		} = options;

		// create entities & actions
		let entity_graph = graph.map(
			|_, actions| {
				let entity = world.spawn((
					Name::from("Action Graph Node"),
					RunTimer::default(),
				));
				if let Some(target) = target {
					world.insert(entity, TargetEntity(target));
				}
				for action in actions.iter() {
					world.apply_action_typed(action, entity);
				}
				entity
			},
			|_, _| (),
		);

		// create edges
		for (index, entity) in Iterator::zip(
			entity_graph.node_indices(),
			entity_graph.node_weights(),
		) {
			let children = entity_graph
				.neighbors_directed_in_order(
					index,
					petgraph::Direction::Outgoing,
				)
				.map(|index| entity_graph[index])
				.collect::<Vec<_>>();
			world.insert(*entity, Edges(children));
		}

		if run_on_spawn {
			if let Some(root) = entity_graph.root() {
				world.insert(*root, Running);
			} else {
				log::warn!("Tried to run on spawn but graph is empty");
			}
		}

		let entity_graph = EntityGraph(entity_graph);
		if let Some(target) = target {
			// NOTE this breaks multiple graphs per target
			world.insert(target, entity_graph.clone());
		}
		entity_graph
	}


	pub fn despawn(&self, commands: &mut Commands) {
		for entity in self.node_weights() {
			commands.entity(*entity).despawn();
		}
	}
}

#[derive(Debug, Default, Clone, Deref, DerefMut, Resource)]
pub struct TrackedEntityGraphs(pub HashMap<Entity, EntityGraph>);

// TODO refactor to allow for multiple graphs
/// This mechanism requires at least one frame between spawning a graph
/// and deleting it
pub fn cleanup_entity_graph(
	mut commands: Commands,
	mut tracked_entities: ResMut<TrackedEntityGraphs>,
	added_graphs: Query<(Entity, &EntityGraph), Changed<EntityGraph>>,
	mut entity_graphs: RemovedComponents<EntityGraph>,
) {
	for (entity, graph) in added_graphs.iter() {
		tracked_entities.insert(entity, graph.clone());
	}

	for entity in entity_graphs.read() {
		if let Some(graph) = tracked_entities.remove(&entity) {
			graph.despawn(&mut commands);
		} else {
			log::warn!("Entity {entity:?} not found in tracked entity graphs");
		}
	}
}
