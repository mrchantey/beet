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

		let actors = self
			.children
			.iter_descendants_inclusive(thread_entity)
			.filter_map(|entity| self.actors.get(entity).ok())
			.collect::<Vec<_>>();

		let actions = actors
			.iter()
			.filter_map(|(_, actor, actions)| {
				actions.map(|actions| {
					actions
						.iter()
						.filter_map(|entity| self.actions.get(entity).ok())
						.map(|(entity, action, res_meta)| {
							(entity, action, *actor, res_meta)
						})
				})
			})
			.flatten()
			.collect::<Vec<_>>();

		ThreadView {
			thread: (thread_entity, thread),
			actors,
			actions,
		}
		.xok()
	}
}


pub struct ThreadView<'a> {
	pub thread: (Entity, &'a Thread),
	/// The list of actors in bfs order of [`Children`]
	pub actors: Vec<(Entity, &'a Actor, Option<&'a Actions>)>,
	/// The list of actions in this thread, sorted chonologically by [`ActionId`]
	pub actions: Vec<(Entity, &'a Action, &'a Actor, Option<&'a ResponseMeta>)>,
}


impl<'a> ThreadView<'a> {
	pub fn thread_id(&self) -> ThreadId { self.thread.1.id() }

	pub fn actor(
		&self,
		action: Entity,
	) -> Result<(Entity, &'a Actor, Option<&'a Actions>)> {
		self.actors
			.iter()
			.find(|(_, _, actions)| {
				actions
					.map(|actions| actions.iter().any(|other| action == other))
					.unwrap_or(false)
			})
			.cloned()
			.ok_or_else(|| {
				bevyhow!(
					"No actor found for action {action:?} in thread {thread:?}",
					thread = self.thread.0
				)
			})
	}
}
