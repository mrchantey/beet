//! Agent resolution for actions in a behavior tree.
//!
//! Actions often need to operate on a single "agent" entity (e.g., the character
//! with a [`Transform`]). This module provides the relationship components and
//! query helper to resolve which entity an action should target.
//!
//! The agent is one structural fact about an action, independent of which
//! components a caller asks for: a relation ([`ActionOf`]), a containment mark
//! ([`AgentRoot`]), else the tree's root.
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
/// Agent targeting and nothing else: call resolution is self-only, so pointing an
/// action at an agent never makes it a dispatch candidate for that agent.
///
/// # Example
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_action::prelude::*;
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

/// Marks an entity as the agent of every action below it.
///
/// Without a mark, an action's agent is its tree's root, which makes an agent
/// subtree uncomposable: mount a control-flow node above a character and the
/// node becomes the root, so every action below resolves to an entity holding
/// none of the components those actions operate on. The mark keeps the subtree
/// working wherever it is mounted, and resolves a nested agent to the nearer
/// one.
///
/// ```bsx
/// <Fallback>
///     <Name("Malenia") {(Health(100.0), AgentRoot, Repeat)}>..</Name>
///     <EndWith/>
/// </Fallback>
/// ```
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct AgentRoot;

/// System parameter for resolving the agent entity of an action.
///
/// This type optionally accepts `QueryData` and `QueryFilter` generics for
/// conveniently querying components on the resolved agent.
///
/// # Agent Resolution
///
/// Walking the ancestors inclusive, the agent is the first of:
/// 1. An [`ActionOf`]'s target
/// 2. An entity marked [`AgentRoot`]
/// 3. The root ancestor
///
/// Every accessor resolves through [`entity`](AgentQuery::entity), so the
/// agent never depends on the components asked for.
///
/// # Example
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_action::prelude::*;
/// fn my_system(agents: AgentQuery<&Transform>) {
/// 	for action in [Entity::PLACEHOLDER] {
/// 		// Get the agent's transform for this action
/// 		if let Ok(transform) = agents.get(action) {
/// 			let _ = transform;
/// 		}
/// 	}
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
	/// Query for [`AgentRoot`] marks.
	pub roots: Query<'w, 's, (), With<AgentRoot>>,
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
	/// Resolution order, walking the ancestors inclusive:
	/// 1. An [`ActionOf`]'s target
	/// 2. An entity marked [`AgentRoot`]
	/// 3. The root ancestor
	pub fn entity(&self, action: Entity) -> Entity {
		// cache root to avoid double traversal
		let mut root = action;
		self.parents
			.iter_ancestors_inclusive(action)
			.find_map(|entity| {
				root = entity;
				match self.actions.get(entity) {
					Ok(action_of) => Some(action_of.get()),
					Err(_) => self.roots.contains(entity).then_some(entity),
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
			.expect(
				"AgentQuery: contains() passed but get_mut() failed, query state changed mid-system",
			)
			.xok()
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use bevy::ecs::system::SystemState;

	#[beet_core::test]
	fn agent_is_action_when_no_parent() {
		let mut world = World::new();
		let action = world.spawn_empty().id();

		let mut state = SystemState::<AgentQuery>::from_world(&mut world);
		let agent_query = state.get(&world).unwrap();

		agent_query.entity(action).xpect_eq(action);
		state.apply(&mut world);
	}

	#[beet_core::test]
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
		let agent_query = state.get(&world).unwrap();

		agent_query.entity(child).xpect_eq(root);
		state.apply(&mut world);
	}

	#[beet_core::test]
	fn agent_is_action_of() {
		let mut world = World::new();
		let agent = world.spawn_empty().id();
		let action = world.spawn(ActionOf(agent)).id();

		let mut state = SystemState::<AgentQuery>::from_world(&mut world);
		let agent_query = state.get(&world).unwrap();

		agent_query.entity(action).xpect_eq(agent);
		state.apply(&mut world);
	}

	#[beet_core::test]
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
		let agent_query = state.get(&world).unwrap();

		// child's agent should be the ActionOf target, not the root
		agent_query.entity(child).xpect_eq(agent);
		// root's agent should also be the ActionOf target
		agent_query.entity(root).xpect_eq(agent);
		state.apply(&mut world);
	}

	/// A marked agent subtree keeps working when something is mounted above
	/// it: a wrapping control-flow node becoming the tree's root does not strip
	/// every action below it of the agent it operates on.
	#[beet_core::test]
	fn agent_is_marked_root() {
		#[derive(Component, PartialEq, Debug)]
		struct Health(u32);

		let mut world = World::new();
		// wrapper -> agent -> action, the shape a `<Fallback>` around a
		// character produces
		let wrapper = world
			.spawn(children![(Health(100), AgentRoot, children![()])])
			.flush();
		let agent = world.entity(wrapper).get::<Children>().unwrap()[0];
		let action = world.entity(agent).get::<Children>().unwrap()[0];

		let mut state =
			SystemState::<AgentQuery<&Health>>::from_world(&mut world);
		let agent_query = state.get(&world).unwrap();

		agent_query.entity(action).xpect_eq(agent);
		agent_query.get(action).unwrap().xpect_eq(Health(100));
		// the mark is itself an agent's action root
		agent_query.entity(agent).xpect_eq(agent);
		state.apply(&mut world);
	}

	/// The relation outranks the mark, whatever it points at.
	#[beet_core::test]
	fn action_of_outranks_mark() {
		let mut world = World::new();
		let agent = world.spawn_empty().id();
		let marked = world
			.spawn((AgentRoot, children![(ActionOf(agent), children![()])]))
			.flush();
		let redirected = world.entity(marked).get::<Children>().unwrap()[0];
		let action = world.entity(redirected).get::<Children>().unwrap()[0];

		let mut state = SystemState::<AgentQuery>::from_world(&mut world);
		let agent_query = state.get(&world).unwrap();

		agent_query.entity(action).xpect_eq(agent);
		state.apply(&mut world);
	}
}
