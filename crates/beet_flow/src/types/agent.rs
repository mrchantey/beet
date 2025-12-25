use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::query::QueryData;
use bevy::ecs::query::QueryEntityError;
use bevy::ecs::query::QueryFilter;
use bevy::ecs::query::ROQueryItem;

/// Declare this action as belonging to the
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

/// Wrap an [`ActionEvent`] specifying the agent entity it should be performed on.
pub struct AgentEvent<E> {
	agent: Option<Entity>,
	event: E,
}
impl<E> AgentEvent<E> {
	pub fn new(agent: Option<Entity>, event: E) -> Self {
		Self { agent, event }
	}
}


#[extend::ext(name=ActionEventAgentExt)]
pub impl<E, T> E
where
	E: 'static
		+ Send
		+ Sync
		+ for<'a> Event<Trigger<'a> = ActionTrigger<false, E, T>>,
	T: 'static + Send + Sync + Traversal<E>,
{
	fn with_agent(self, agent: Entity) -> AgentEvent<E> {
		AgentEvent::new(Some(agent), self)
	}
	fn with_agent_opt(self, agent: Option<Entity>) -> AgentEvent<E> {
		AgentEvent::new(agent, self)
	}
}

impl<E, T> IntoEntityTargetEvent<(T, Self)> for AgentEvent<E>
where
	E: 'static
		+ Send
		+ Sync
		+ for<'a> Event<Trigger<'a> = ActionTrigger<false, E, T>>,
	T: 'static + Send + Sync + Traversal<E>,
{
	type Event = E;
	type Trigger = ActionTrigger<false, E, T>;

	fn into_entity_target_event(
		self,
		entity: Entity,
	) -> (Self::Event, Self::Trigger) {
		let cx = match self.agent {
			Some(agent) => ActionContext::new_with_agent(entity, agent),
			None => ActionContext::new_no_agent(entity),
		};
		(self.event, ActionTrigger::new(cx))
	}
}



/// A [`SystemParam`] used to get the agent for a particular action.
/// This type optionally accepts a `QueryData` and `QueryFilter` for conveniently getting
/// components of the agent.
/// See [`AgentQuery::entity`] for how the entity is resolved.
// TODO Running should track ActionContext::agent
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
impl<'w, 's, D, F> AgentQuery<'w, 's, D, F>
where
	D: 'static + QueryData,
	F: 'static + QueryFilter,
{
	/// Get the 'agent' entity for this action.
	/// The agent is resolved in the following order:
	/// - The first [`ActionOf`] in ancestors (inclusive)
	/// - The root ancestor
	/// currently this does NOT track the ActionContext::agent
	// TODO track ActionContext::agent
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
