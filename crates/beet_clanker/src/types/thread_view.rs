use crate::prelude::*;
use beet_core::prelude::*;

#[derive(SystemParam)]
pub struct ThreadQuery<'w, 's> {
	pub commands: Commands<'w, 's>,
	pub ancestors: Query<'w, 's, &'static ChildOf>,
	pub children: Query<'w, 's, &'static Children>,
	pub threads: Query<'w, 's, (Entity, &'static Thread)>,
	pub action_ofs: Query<'w, 's, &'static ActionOf>,
	pub actors:
		Query<'w, 's, (Entity, &'static Actor, Option<&'static Actions>)>,
	pub actions:
		Query<'w, 's, (Entity, &'static Action, Option<&'static ResponseMeta>)>,
}

impl<'w, 's> ThreadQuery<'w, 's> {
	/// Recurse up ancestors to find the [`Thread`] entity,
	/// then create a corresponding [`ThreadView`].
	/// Valid positions are:
	/// - any descendant of a thread, ie an Actor
	/// - any `ActionOf`
	pub fn view(&self, mut entity: Entity) -> Result<ThreadView<'_>> {
		// handle the action position
		if let Ok(actor) = self.action_ofs.get(entity) {
			entity = actor.get()
		};

		let (thread_entity, thread) = self
			.ancestors
			.iter_ancestors_inclusive(entity)
			.find_map(|ancestor| self.threads.get(ancestor).ok())
			.ok_or_else(|| bevyhow!("No Thread in ancestors of {entity:?}"))?;

		let actors: Vec<ActorView<'_>> = self
			.children
			.iter_descendants_inclusive(thread_entity)
			.filter_map(|entity| self.actors.get(entity).ok())
			.map(|(entity, actor, actions)| ActorView {
				entity,
				actor,
				actions,
			})
			.collect();

		let mut actions: Vec<ActionView<'_>> = actors
			.iter()
			.filter_map(|av| {
				av.actions.map(|actions| {
					actions
						.iter()
						.filter_map(|entity| self.actions.get(entity).ok())
						.map(|(entity, action, response_meta)| ActionView {
							entity,
							action,
							actor: av.actor,
							actor_entity: av.entity,
							response_meta,
						})
				})
			})
			.flatten()
			.collect();
		actions.sort_by_key(|av| av.action.id());

		ThreadView {
			entity: thread_entity,
			thread,
			actors,
			actions,
		}
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

	/// Find the [`ActorView`] that owns the given action entity.
	pub fn actor_from_action_entity(
		&self,
		action: Entity,
	) -> Result<&ActorView<'a>> {
		self.actors
			.iter()
			.find(|av| {
				av.actions
					.map(|actions| actions.iter().any(|other| action == other))
					.unwrap_or(false)
			})
			.ok_or_else(|| {
				bevyhow!(
					"No actor found for action {action:?} in thread {thread:?}",
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
	pub actions: Option<&'a Actions>,
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
	pub fn actor_id(&self) -> ActorId { self.actor.id() }
}
