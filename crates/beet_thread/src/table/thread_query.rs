use crate::prelude::*;
use beet_core::prelude::*;

/// Read access to a [`Thread`] and its [`ThreadWindow`] from anywhere in the
/// thread's entity tree.
///
/// Conversation reads go through the in-memory window ([`Self::window`]); this
/// param only locates the thread entity and reads behavior-entity identity
/// ([`ActorRef`]), tools, and tool choice.
#[derive(SystemParam)]
pub struct ThreadQuery<'w, 's> {
	pub ancestors: Query<'w, 's, &'static ChildOf>,
	pub children: Query<'w, 's, &'static Children>,
	pub threads:
		Query<'w, 's, (Entity, &'static Thread, &'static ThreadWindow)>,
	pub actor_refs: Query<'w, 's, &'static ActorRef>,
	pub tool_choices: Query<'w, 's, &'static ToolChoice>,
	pub tools: Query<'w, 's, (Entity, &'static ToolDefinition)>,
}

impl<'w, 's> ThreadQuery<'w, 's> {
	/// Walk ancestors to the [`Thread`] entity and its component.
	pub fn thread_entity(&self, entity: Entity) -> Result<(Entity, &Thread)> {
		self.ancestors
			.iter_ancestors_inclusive(entity)
			.find_map(|ancestor| {
				self.threads
					.get(ancestor)
					.ok()
					.map(|(entity, thread, _)| (entity, thread))
			})
			.ok_or_else(|| bevyhow!("No Thread in ancestors of {entity}"))
	}

	/// The [`Thread`], its entity, and its [`ThreadWindow`].
	pub fn thread_and_window(
		&self,
		entity: Entity,
	) -> Result<(Entity, &Thread, &ThreadWindow)> {
		self.ancestors
			.iter_ancestors_inclusive(entity)
			.find_map(|ancestor| self.threads.get(ancestor).ok())
			.ok_or_else(|| bevyhow!("No Thread in ancestors of {entity}"))
	}

	/// The thread's window.
	pub fn window(&self, entity: Entity) -> Result<&ThreadWindow> {
		self.thread_and_window(entity).map(|(_, _, window)| window)
	}

	/// The [`ActorId`] a behavior entity acts as.
	pub fn actor_id(&self, entity: Entity) -> Result<ActorId> {
		self.actor_refs
			.get(entity)
			.map(|actor_ref| **actor_ref)
			.map_err(|_| bevyhow!("entity {entity} has no ActorRef"))
	}

	/// The [`ToolChoice`] on a behavior entity, if any.
	pub fn tool_choice(&self, entity: Entity) -> Option<&ToolChoice> {
		self.tool_choices.get(entity).ok()
	}

	/// Recurse down (DFS) to find all [`ToolDefinition`] entities, returning
	/// their entities and definitions.
	pub fn tools(&self, actor: Entity) -> Vec<(Entity, &ToolDefinition)> {
		self.children
			.iter_descendants_inclusive(actor)
			.filter_map(|entity| self.tools.get(entity).ok())
			.collect()
	}
}
