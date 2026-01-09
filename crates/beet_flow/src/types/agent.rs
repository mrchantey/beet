use beet_core::prelude::*;
use bevy::ecs::query::QueryData;
use bevy::ecs::query::QueryEntityError;
use bevy::ecs::query::QueryFilter;
use bevy::ecs::query::ROQueryItem;

/// Declare this action as belonging to the specified agent entity.
#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = Actions)]
pub struct ActionOf(pub Entity);

/// Added to agents, listing all actions which belong to it. Actions which are
/// [`Children`].
#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = ActionOf, linked_spawn)]
pub struct Actions(Vec<Entity>);


/// A [`SystemParam`] used to get the agent for a particular action.
/// This type optionally accepts a `QueryData` and `QueryFilter` for conveniently getting
/// components of the agent.
/// See [`AgentQuery::entity`] for how the entity is resolved.
#[derive(SystemParam)]
pub struct AgentQuery<'w, 's, D = (), F = ()>
where
	D: 'static + QueryData,
	F: 'static + QueryFilter,
{
	/// A [`ChildOf`] query
	pub parents: Query<'w, 's, &'static ChildOf>,
	/// A [`Children`] query
	pub children: Query<'w, 's, &'static Children>,
	/// A [`ActionOf`] query
	pub actions: Query<'w, 's, &'static ActionOf>,
	/// A user defined query
	pub query: Query<'w, 's, D, F>,
}


impl AgentQuery<'_, '_, (), ()> {
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
	/// Get the 'agent' entity for this action.
	/// The agent is resolved in the following order:
	/// - The first [`ActionOf`] in ancestors (inclusive)
	/// - The root ancestor
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

	/// Get the query item for this `agent`
	pub fn get(
		&self,
		action: Entity,
	) -> Result<ROQueryItem<'_, 's, D>, QueryEntityError> {
		let agent = self.entity(action);
		self.query.get(agent)
	}

	pub fn contains(&self, entity: Entity) -> bool {
		let agent = self.entity(entity);
		self.query.contains(agent)
	}

	/// Get the query item for this `agent`
	pub fn get_mut(
		&mut self,
		entity: Entity,
	) -> Result<D::Item<'_, 's>, QueryEntityError> {
		let agent = self.entity(entity);
		self.query.get_mut(agent)
	}

	/// Get the item for this `agent`
	/// or its first matching child (BFS)
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

	/// Get the query item for this `agent`
	/// or its first matching child (BFS)
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
	}

	#[test]
	fn agent_is_action_of() {
		let mut world = World::new();
		let agent = world.spawn_empty().id();
		let action = world.spawn(ActionOf(agent)).id();

		let mut state = SystemState::<AgentQuery>::from_world(&mut world);
		let agent_query = state.get(&world);

		agent_query.entity(action).xpect_eq(agent);
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
	}
}
