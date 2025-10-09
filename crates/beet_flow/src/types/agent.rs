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
/// This event type, paired with [`GlobalAgentQuery`], enables 'global control flow',
/// where a single tree of observers can be reused for multiple agents.
/// This is particularly useful for agents which are frequently spawned/despawned as it
/// avoids creating a new tree for each entity.
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
		entity: &mut EntityWorldMut,
	) -> (Self::Event, Self::Trigger) {
		let cx = match self.agent {
			Some(agent) => ActionContext::new_with_agent(entity.id(), agent),
			None => ActionContext::new(entity),
		};
		(self.event, ActionTrigger::new(cx))
	}
}



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
impl<'w, 's, D, F> AgentQuery<'w, 's, D, F>
where
	D: 'static + QueryData,
	F: 'static + QueryFilter,
{
	/// Get the 'agent' entity for this action.
	/// The agent is resolved in the following order:
	/// - The first [`ActionOf`] in ancestors (inclusive)
	/// - The root ancestor
	pub fn entity(&self, entity: Entity) -> Entity {
		// cache root to avoid double traversal
		let mut root = entity;
		self.parents
			.iter_ancestors_inclusive(entity)
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
		entity: Entity,
	) -> Result<ROQueryItem<'_, 's, D>, QueryEntityError> {
		let agent = self.entity(entity);
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


/// A wrapper for [`AgentQuery`] that first checks the [`ActionTrigger::agent`] to resolve
/// the agent entity. For more info see [`AgentEvent`]
#[derive(SystemParam)]
pub struct GlobalAgentQuery<'w, 's, D = (), F = ()>
where
	D: 'static + QueryData,
	F: 'static + QueryFilter,
{
	agent_query: AgentQuery<'w, 's, D, F>,
}
impl<'w, 's, D, F> std::ops::Deref for GlobalAgentQuery<'w, 's, D, F>
where
	D: 'static + QueryData,
	F: 'static + QueryFilter,
{
	type Target = AgentQuery<'w, 's, D, F>;
	fn deref(&self) -> &Self::Target { &self.agent_query }
}

impl<'w, 's, D, F> std::ops::DerefMut for GlobalAgentQuery<'w, 's, D, F>
where
	D: 'static + QueryData,
	F: 'static + QueryFilter,
{
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.agent_query }
}


impl<'w, 's, D, F> GlobalAgentQuery<'w, 's, D, F>
where
	D: 'static + QueryData,
	F: 'static + QueryFilter,
{
	/// Get the 'agent' entity for this action.
	/// The agent is resolved in the following order:
	/// - [`ActionTrigger::agent`]
	/// - [`AgentQuery::entity`]
	pub fn entity(&self, ev: &On<impl ActionEvent>) -> Entity { ev.agent() }

	/// Get the query item for this `agent`
	pub fn get(
		&self,
		ev: &On<impl ActionEvent>,
	) -> Result<ROQueryItem<'_, 's, D>, QueryEntityError> {
		let agent = self.entity(ev);
		self.query.get(agent)
	}

	/// Get the query item for this `agent`
	pub fn get_mut(
		&mut self,
		ev: &On<impl ActionEvent>,
	) -> Result<D::Item<'_, 's>, QueryEntityError> {
		let agent = self.entity(ev);
		self.query.get_mut(agent)
	}

	/// Get the item for this `agent`
	/// or its first matching child (BFS)
	pub fn get_descendent(
		&self,
		ev: &On<impl ActionEvent>,
	) -> Result<ROQueryItem<'_, 's, D>> {
		let agent = self.entity(ev);
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
		ev: &On<impl ActionEvent>,
	) -> Result<D::Item<'_, 's>> {
		let agent = self.entity(ev);
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
