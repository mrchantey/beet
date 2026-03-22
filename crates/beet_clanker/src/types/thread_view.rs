use crate::prelude::*;
use beet_core::prelude::*;

#[derive(SystemParam)]
pub struct ThreadQuery<'w, 's> {
	pub commands: Commands<'w, 's>,
	pub ancestors: Query<'w, 's, &'static ChildOf>,
	pub children: Query<'w, 's, &'static Children>,
	pub threads: Query<'w, 's, (Entity, &'static Thread)>,
	pub actors: Query<'w, 's, (Entity, &'static Actor)>,
	pub actions:
		Query<'w, 's, (Entity, &'static Action, Option<&'static ResponseMeta>)>,
}

impl<'w, 's> ThreadQuery<'w, 's> {
	/// Recurse up ancestors to find the [`Thread`] entity,
	/// then create a corresponding [`ThreadView`].
	/// Valid positions are:
	/// - any descendant of a thread, ie an Actor
	/// - any `ActionOf`
	pub fn view(&self, entity: Entity) -> Result<ThreadView<'_>> {
		let (thread_entity, thread) = self
			.ancestors
			.iter_ancestors_inclusive(entity)
			.find_map(|ancestor| self.threads.get(ancestor).ok())
			.ok_or_else(|| bevyhow!("No Thread in ancestors of {entity:?}"))?;

		let actors: Vec<ActorView<'_>> = self
			.children
			.iter_descendants_inclusive(thread_entity)
			.filter_map(|entity| self.actors.get(entity).ok())
			.map(|(entity, actor)| ActorView { entity, actor })
			.collect();

		let mut actions: Vec<ActionView<'_>> = self
			.children
			.iter_descendants_inclusive(thread_entity)
			.filter_map(|entity| self.actions.get(entity).ok())
			.xtry_map(
				|(entity, action, response_meta)| -> Result<ActionView> {
					let actor = self.actor_from_action_entity(entity)?;
					ActionView {
						entity,
						action,
						actor: actor.actor,
						actor_entity: actor.entity,
						response_meta,
					}
					.xok()
				},
			)?;
		actions.sort_by_key(|av| av.action.id());

		ThreadView {
			entity: thread_entity,
			thread,
			actors,
			actions,
		}
		.xok()
	}

	/// Find the [`ActorView`] that owns the given action entity.
	pub fn actor_from_action_entity<'a>(
		&'a self,
		action: Entity,
	) -> Result<ActorView<'a>> {
		self.ancestors
			.iter_ancestors_inclusive(action)
			.find_map(|entity| self.actors.get(entity).ok())
			.map(|(entity, actor)| ActorView { entity, actor })
			.ok_or_else(|| {
				bevyhow!("No actor ancestor found for action {action:?}")
			})
	}

	pub fn spawn_action(
		&mut self,
		parent: Entity,
		status: ActionStatus,
		payload: impl Into<ActionPayload>,
	) -> Result<Entity> {
		let actor_id = self
			.ancestors
			.iter_ancestors_inclusive(parent)
			.find_map(|entity| {
				self.actors.get(entity).map(|(_, actor)| actor.id()).ok()
			})
			.ok_or_else(|| {
				bevyhow!("No actor ancestor found for {parent:?}")
			})?;
		let thread_id = self
			.ancestors
			.iter_ancestors_inclusive(parent)
			.find_map(|entity| {
				self.threads.get(entity).map(|(_, thread)| thread.id()).ok()
			})
			.ok_or_else(|| {
				bevyhow!("No thread ancestor found for {parent:?}")
			})?;
		self.commands
			.spawn((
				ChildOf(parent),
				Action::new(actor_id, thread_id, status, payload),
			))
			.id()
			.xok()
	}
}
#[derive(Debug, Clone)]
pub struct ThreadView<'a> {
	pub entity: Entity,
	pub thread: &'a Thread,
	/// The list of actors in bfs order of [`Children`]
	pub actors: Vec<ActorView<'a>>,
	/// The list of actions in this thread, sorted chronologically by [`ActionId`]
	pub actions: Vec<ActionView<'a>>,
}

impl std::ops::Deref for ThreadView<'_> {
	type Target = Thread;
	fn deref(&self) -> &Self::Target { self.thread }
}


impl<'a> ThreadView<'a> {
	pub fn id(&self) -> ThreadId { self.thread.id() }

	pub fn actor(&self, actor_entity: Entity) -> Result<&ActorView<'a>> {
		self.actors
			.iter()
			.find(|av| av.entity == actor_entity)
			.ok_or_else(|| {
				bevyhow!(
					"No actor for entity {actor_entity} found in thread {thread:?}",
					thread = self.thread
				)
			})
	}
	pub fn actor_from_id(&self, actor_id: ActorId) -> Result<&ActorView<'a>> {
		self.actors
			.iter()
			.find(|av| av.actor.id() == actor_id)
			.ok_or_else(|| {
				bevyhow!(
					"No actor with id {actor_id} found in thread {thread:?}",
					thread = self.thread
				)
			})
	}
	pub fn action_from_id(
		&self,
		action_id: ActionId,
	) -> Result<&ActionView<'a>> {
		self.actions
			.iter()
			.find(|av| av.action.id() == action_id)
			.ok_or_else(|| {
				bevyhow!(
					"No action with id {action_id} found in thread {thread:?}",
					thread = self.thread
				)
			})
	}

	/// Find a stored [`ResponseMeta`] for the given actor, provider, and model.
	pub fn stored_response(
		&self,
		actor: Entity,
		provider_slug: &str,
		model_slug: &str,
	) -> Option<(&ActionView<'_>, &ResponseMeta)> {
		self.actions.iter().find_map(|av| {
			if av.actor_entity != actor {
				return None;
			}
			let meta = av.response_meta?;
			(meta.provider_slug == provider_slug
				&& meta.model_slug == model_slug
				&& meta.response_stored)
				.then_some((av, meta))
		})
	}

	pub fn actions_from(
		&self,
		after_action: Option<ActionId>,
	) -> Vec<ActionView<'_>> {
		if let Some(after) = after_action {
			match self.actions.iter().position(|a| a.id() == after) {
				Some(i) => self.actions[i + 1..].to_vec(),
				None => self.actions.clone(),
			}
		} else {
			self.actions.clone()
		}
	}
}

#[derive(Debug, Clone)]
pub struct ActorView<'a> {
	pub entity: Entity,
	pub actor: &'a Actor,
}

impl std::ops::Deref for ActorView<'_> {
	type Target = Actor;
	fn deref(&self) -> &Self::Target { self.actor }
}

#[derive(Debug, Clone)]
pub struct ActionView<'a> {
	pub entity: Entity,
	pub actor_entity: Entity,
	pub action: &'a Action,
	pub actor: &'a Actor,
	pub response_meta: Option<&'a ResponseMeta>,
}


impl std::ops::Deref for ActionView<'_> {
	type Target = Action;
	fn deref(&self) -> &Self::Target { self.action }
}

impl ActionView<'_> {
	pub fn entity(&self) -> Entity { self.entity }
	pub fn actor_id(&self) -> ActorId { self.actor.id() }
}
