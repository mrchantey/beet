//! Agent resolution for actions in a behavior tree.
//!
//! Actions often need to operate on a single "agent" entity (e.g., the character
//! with a [`Transform`]). This module provides the relationship components and
//! query helper to resolve which entity an action should target.
use beet_core::prelude::*;
use bevy::ecs::query::QueryData;
use bevy::ecs::query::QueryEntityError;
use bevy::ecs::query::QueryFilter;
use bevy::ecs::query::ROQueryItem;

/// Declares that this action belongs to a specific agent entity.
///
/// When an action needs to target a specific entity that isn't its root
/// ancestor, use this component to specify the relationship explicitly.
///
/// # Example
///
/// ```
/// # use bevy::prelude::*;
/// # use beet_flow::prelude::*;
/// # let mut world = World::new();
/// // The agent entity (e.g., a character)
/// let agent = world.spawn(Transform::default()).id();
///
/// // An action that belongs to this agent
/// let action = world.spawn(ActionOf(agent)).id();
/// ```
#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = Actions)]
pub struct ActionOf(pub Entity);

/// Component added to agents listing all actions that belong to it.
///
/// This is automatically managed by the [`ActionOf`] relationship.
#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = ActionOf, linked_spawn)]
pub struct Actions(Vec<Entity>);


/// System parameter for resolving the agent entity of an action.
///
/// This type optionally accepts `QueryData` and `QueryFilter` generics for
/// conveniently querying components on the resolved agent.
///
/// # Agent Resolution
///
/// The agent is resolved in this order (see [`AgentQuery::entity`]):
/// 1. The first [`ActionOf`] relationship found in ancestors (inclusive)
/// 2. The root ancestor if no [`ActionOf`] is found
///
/// # Example
///
/// ```ignore
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// fn my_action_system(
///     ev: On<GetOutcome>,
///     agents: AgentQuery<&Transform>,
/// ) {
///     // Get the agent's transform for this action
///     if let Ok(transform) = agents.get(ev.target()) {
///         // Use the transform...
///     }
/// }
/// ```
#[derive(SystemParam)]
pub struct AgentQuery<'w, 's, D = (), F = ()>
where
	D: 'static + QueryData,
	F: 'static + QueryFilter,
{
	/// Query for [`ChildOf`] relationships used in ancestor traversal.
	pub parents: Query<'w, 's, &'static ChildOf>,
	/// Query for [`Children`] used in descendant iteration.
	pub children: Query<'w, 's, &'static Children>,
	/// Query for [`ActionOf`] relationships.
	pub actions: Query<'w, 's, &'static ActionOf>,
	/// Query for entities with [`Actions`] component.
	pub agents: Query<'w, 's, &'static Actions>,
	/// User-defined query for agent components.
	pub query: Query<'w, 's, D, F>,
}


impl AgentQuery<'_, '_, (), ()> {
	/// Resolves the agent entity asynchronously.
	pub async fn entity_async(world: &AsyncWorld, action: Entity) -> Entity {
		world
			.run_system_cached_with(
				|In(action): In<Entity>, query: AgentQuery| {
					query.entity(action)
				},
				action,
			)
			.await
			.unwrap()
	}
}


impl<'w, 's, D, F> AgentQuery<'w, 's, D, F>
where
	D: 'static + QueryData,
	F: 'static + QueryFilter,
{
	/// Returns the agent entity for the given action.
	///
	/// Resolution order:
	/// 1. First [`ActionOf`] found in ancestors (inclusive)
	/// 2. Root ancestor if no [`ActionOf`] exists
	pub fn entity(&self, action: Entity) -> Entity {
		// cache root to avoid double traversal
		let mut root = action;
		self.parents
			.iter_ancestors_inclusive(action)
			.find_map(|entity| {
				root = entity;
				if let Ok(action_of) = self.actions.get(entity) {
					Some(action_of.get())
				} else {
					None
				}
			})
			.unwrap_or(root)
	}
	/// Returns `true` if the agent matches the query filter.
	pub fn contains(&self, entity: Entity) -> bool {
		let agent = self.entity(entity);
		self.query.contains(agent)
	}

	/// Returns the query item for the agent of the given action.
	pub fn get(
		&self,
		action: Entity,
	) -> Result<ROQueryItem<'_, 's, D>, QueryEntityError> {
		let agent = self.entity(action);
		self.query.get(agent)
	}


	/// Returns the mutable query item for the agent of the given action.
	pub fn get_mut(
		&mut self,
		entity: Entity,
	) -> Result<D::Item<'_, 's>, QueryEntityError> {
		let agent = self.entity(entity);
		self.query.get_mut(agent)
	}

	/// Returns the query item for the agent or its first matching descendant (BFS).
	pub fn get_descendent(
		&self,
		entity: Entity,
	) -> Result<ROQueryItem<'_, 's, D>> {
		let agent = self.entity(entity);
		self.children
			.iter_descendants_inclusive(agent)
			.find_map(|entity| self.query.get(entity).ok())
			.ok_or_else(|| {
				bevyhow!("No entity in agent descendents matches the query")
			})
	}

	/// Returns the mutable query item for the agent or its first matching descendant (BFS).
	pub fn get_descendent_mut(
		&mut self,
		entity: Entity,
	) -> Result<D::Item<'_, 's>> {
		let agent = self.entity(entity);
		self.children
			.iter_descendants_inclusive(agent)
			.find(|entity| self.query.contains(*entity))
			.ok_or_else(|| {
				bevyhow!("No entity in agent descendents matches the query")
			})?
			.xmap(|entity| self.query.get_mut(entity))
			.unwrap()
			.xok()
	}
}


#[cfg(test)]
mod test {
	use super::*;
	use bevy::ecs::system::SystemState;

	#[test]
	fn agent_is_action_when_no_parent() {
		let mut world = World::new();
		let action = world.spawn_empty().id();

		let mut state = SystemState::<AgentQuery>::from_world(&mut world);
		let agent_query = state.get(&world);

		agent_query.entity(action).xpect_eq(action);
		state.apply(&mut world);
	}

	#[test]
	fn agent_is_root_ancestor() {
		let mut world = World::new();
		let root = world.spawn(children![()]).flush();

		let child = world
			.query::<&Children>()
			.single(&world)
			.unwrap()
			.iter()
			.next()
			.unwrap();

		let mut state = SystemState::<AgentQuery>::from_world(&mut world);
		let agent_query = state.get(&world);

		agent_query.entity(child).xpect_eq(root);
		state.apply(&mut world);
	}

	#[test]
	fn agent_is_action_of() {
		let mut world = World::new();
		let agent = world.spawn_empty().id();
		let action = world.spawn(ActionOf(agent)).id();

		let mut state = SystemState::<AgentQuery>::from_world(&mut world);
		let agent_query = state.get(&world);

		agent_query.entity(action).xpect_eq(agent);
		state.apply(&mut world);
	}

	#[test]
	fn agent_is_ancestor_action_of() {
		let mut world = World::new();
		let agent = world.spawn_empty().id();
		let root = world.spawn((ActionOf(agent), children![()])).flush();

		let child = world
			.query::<&Children>()
			.single(&world)
			.unwrap()
			.iter()
			.next()
			.unwrap();

		let mut state = SystemState::<AgentQuery>::from_world(&mut world);
		let agent_query = state.get(&world);

		// child's agent should be the ActionOf target, not the root
		agent_query.entity(child).xpect_eq(agent);
		// root's agent should also be the ActionOf target
		agent_query.entity(root).xpect_eq(agent);
		state.apply(&mut world);
	}
}
