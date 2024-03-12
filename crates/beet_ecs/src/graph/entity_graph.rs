use crate::prelude::*;
use bevy_core::Name;
use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::prelude::*;
use bevy_reflect::Reflect;
use bevy_utils::prelude::default;
use petgraph::graph::DiGraph;
use serde::Deserialize;
use serde::Serialize;

/// Marker to identify the root of a behavior graph
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component)]
pub struct BehaviorGraphRoot;


#[derive(
	Debug, Default, Clone, Deref, DerefMut, Serialize, Deserialize, Component,
)]
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
	// pub fn from_prefab<T: ActionTypes>(prefab: BehaviorGraphPrefab<T>) -> Self {
	// 	let mut this = EntityGraph::default();
	// 	let root = prefab.root();
	// 	this.add_recursive(&prefab.world, root);
	// 	this
	// }
	// fn add_recursive(&mut self, world: &World, parent: Entity) -> NodeIndex {
	// 	let node_index = self.add_node(parent);
	// 	if let Some(children) = world.get::<Edges>(parent) {
	// 		for child in children.iter() {
	// 			let child_index = self.add_recursive(world, *child);
	// 			self.add_edge(node_index, child_index, ());
	// 		}
	// 	}
	// 	node_index
	// }
	pub fn spawn(
		world: &mut impl WorldOrCommands,
		graph: impl Into<WillyBehaviorGraph>,
		target: Entity,
	) -> Self {
		Self::spawn_with_options(world, graph, EntityGraphOptions {
			target: Some(target),
			..default()
		})
	}
	/// Choosing no target agent means its your responsibility to ensure that all actions in the behavior graph
	/// are compatible, actions that expect an agent may do nothing or panic.
	pub fn spawn_no_target(
		world: &mut impl WorldOrCommands,
		graph: impl Into<WillyBehaviorGraph>,
	) -> Self {
		Self::spawn_with_options(world, graph, EntityGraphOptions {
			target: None,
			..default()
		})
	}

	pub fn spawn_with_options(
		world: &mut impl WorldOrCommands,
		graph: impl Into<WillyBehaviorGraph>,
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
					world.insert(entity, TargetAgent(target));
				}
				for action in actions.actions.iter() {
					world.insert_action(entity, action.as_ref());
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
				world.insert(*root, (BehaviorGraphRoot, Running));
			} else {
				log::warn!("Tried to run on spawn but graph is empty");
			}
		}

		let entity_graph = EntityGraph(entity_graph);
		if let Some(target) = target {
			world.insert(target, AgentMarker);
		}
		entity_graph
	}


	pub fn despawn(&self, commands: &mut Commands) {
		for entity in self.node_weights() {
			commands.entity(*entity).despawn();
		}
	}
}

/// Removes all nodes with a [`TargetAgent`] component that matches the removed agent
pub fn cleanup_entity_graph(
	mut commands: Commands,
	nodes: Query<(Entity, &TargetAgent)>,
	mut removed_agents: RemovedComponents<AgentMarker>,
) {
	for agent in removed_agents.read() {
		for (node, target) in nodes.iter() {
			if **target == agent {
				commands.entity(node).despawn();
			}
		}
	}
}
