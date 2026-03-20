use crate::prelude::*;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;

/// An in-memory unindexed table store for short-lived queries.
/// Correctness is prioritized over efficiencty, ie no indexes are
/// maintained, and actions are sorted per each 'get'.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, Resource,
)]
pub struct ContextMap {
	actors: DocMap<Actor>,
	actions: DocMap<Action>,
	threads: DocMap<Thread>,
}


impl ContextMap {
	pub fn actors(&self) -> &DocMap<Actor> { &self.actors }
	pub fn actors_mut(&mut self) -> &mut DocMap<Actor> { &mut self.actors }

	pub fn actions(&self) -> &DocMap<Action> { &self.actions }
	pub fn actions_mut(&mut self) -> &mut DocMap<Action> { &mut self.actions }

	pub fn threads(&self) -> &DocMap<Thread> { &self.threads }
	pub fn threads_mut(&mut self) -> &mut DocMap<Thread> { &mut self.threads }

	/// Returns all actions belonging to the given thread, sorted chronologically.
	pub fn thread_actions(
		&self,
		thread_id: ThreadId,
		actions_after: Option<ActionId>,
	) -> Vec<&Action> {
		let mut actions: Vec<&Action> = self
			.actions
			.values()
			.filter(|action| action.thread() == thread_id)
			.collect();
		actions.sort_by_key(|action| action.id());

		if let Some(after) = actions_after {
			let pos = actions
				.iter()
				.position(|action| action.id() == after)
				.map(|i| i + 1)
				.unwrap_or(0);
			actions[pos..].to_vec()
		} else {
			actions
		}
	}
}


#[derive(SystemParam)]
pub struct ContextQuery<'w, 's> {
	pub commands: Commands<'w, 's>,
	pub context_map: ResMut<'w, ContextMap>,
	pub ancestors: Query<'w, 's, &'static ChildOf>,
	pub children: Query<'w, 's, &'static Children>,
	pub actor_query: Query<'w, 's, (Entity, &'static ActorId)>,
	pub thread_query: Query<'w, 's, (Entity, &'static ThreadId)>,
}

impl std::ops::Deref for ContextQuery<'_, '_> {
	type Target = ContextMap;
	fn deref(&self) -> &Self::Target { &self.context_map }
}
impl std::ops::DerefMut for ContextQuery<'_, '_> {
	fn deref_mut(&mut self) -> &mut Self::Target { self.context_map.as_mut() }
}

impl ContextQuery<'_, '_> {
	pub fn actor_entities(&self, actor_id: ActorId) -> Vec<Entity> {
		self.actor_query
			.iter()
			.filter_map(|(entity, other_id)| match &actor_id == other_id {
				true => Some(entity),
				false => None,
			})
			.collect()
	}

	pub fn response_complete(
		&mut self,
		response_id: impl Into<String>,
		interrupted: bool,
	) {
		self.commands.trigger(ResponseComplete {
			id: response_id.into(),
			interrupted,
		});
	}

	/// Insert actions into the map and trigger creation/update events.
	/// For actions already in the map (ie from [`PartialItemMap::apply_actions`]),
	/// use [`handle_action_changes`] directly.
	pub fn add_actions<M>(
		&mut self,
		actions: impl XIntoIterator<M, Action>,
	) -> Result<()> {
		let mut changes = ActionChanges::default();
		for action in actions.xinto_iter() {
			let action_id = action.id();
			let exists = self.actions.contains_key(action_id);
			self.actions.insert(action);
			if exists {
				changes.modified.push(action_id);
			} else {
				changes.created.push(action_id);
			}
		}
		self.handle_action_changes(changes)
	}

	/// Trigger creation/update events for the given action changes.
	/// Actions must already exist in the action map.
	pub fn handle_action_changes(&mut self, changes: ActionChanges) -> Result {
		if changes.is_empty() {
			return Ok(());
		}

		for &action_id in changes.all_actions() {
			let action = self.context_map.actions.get(action_id)?;
			let thread_id = action.thread();
			let actor_id = action.author();

			let is_created = changes.created.contains(&action_id);

			if is_created {
				self.commands.trigger(ActionCreated {
					action: action_id,
					thread: thread_id,
					actor: actor_id,
				});
			}

			self.commands.trigger(ActionUpdated {
				action: action_id,
				thread: thread_id,
				actor: actor_id,
			});
		}

		Ok(())
	}
}

#[derive(Default)]
pub struct ActionChanges {
	pub created: Vec<ActionId>,
	pub modified: Vec<ActionId>,
}

impl ActionChanges {
	pub fn is_empty(&self) -> bool {
		self.created.is_empty() && self.modified.is_empty()
	}

	/// All action ids that were either created or modified
	pub fn all_actions(&self) -> impl Iterator<Item = &ActionId> {
		self.created.iter().chain(self.modified.iter())
	}
}

/// Action created event, runs before [`EntityActionCreated`] and [`ActionUpdated`]
#[derive(Event)]
pub struct ActionCreated {
	pub action: ActionId,
	pub thread: ThreadId,
	pub actor: ActorId,
}

#[derive(Event)]
pub struct ActionUpdated {
	pub action: ActionId,
	pub thread: ThreadId,
	pub actor: ActorId,
}

#[derive(Event)]
pub struct ResponseComplete {
	/// The openresponses id for this response
	pub id: String,
	pub interrupted: bool,
}
