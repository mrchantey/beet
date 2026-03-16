use crate::prelude::*;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Serialize,
	Deserialize,
	Component,
)]
#[component(on_add=on_add)]
pub struct Actor {
	/// The name of the actor used when building context
	name: String,
	id: ActorId,
	kind: ActorKind,
	items: Vec<ItemId>,
}

fn on_add(mut world: DeferredWorld, cx: HookContext) {
	let actor_id = world.entity(cx.entity).get::<Actor>().unwrap().id;
	world
		.resource_mut::<ContextMap>()
		.add_actor(actor_id, cx.entity);
}

impl Actor {
	pub fn new(name: impl AsRef<str>, kind: ActorKind) -> Self {
		Self {
			name: name.as_ref().to_string(),
			kind,
			id: ActorId::default(),
			items: Vec::new(),
		}
	}
	pub fn system() -> Self { Self::new("System", ActorKind::System) }
	pub fn developer() -> Self { Self::new("Developer", ActorKind::Developer) }
	pub fn user() -> Self { Self::new("User", ActorKind::Human) }
	pub fn clanker() -> Self { Self::new("Clanker", ActorKind::Agent) }

	/// Get the message role for this actor, relative to the given agent ID.
	/// This is useful when an agent is constructing its context for an
	/// openresponses request.
	pub fn relative_message_role(
		&self,
		agent_id: ActorId,
	) -> openresponses::MessageRole {
		use openresponses::MessageRole;
		match self.kind {
			ActorKind::System => MessageRole::System,
			ActorKind::Developer => MessageRole::Developer,
			ActorKind::Human => MessageRole::User,
			ActorKind::Agent => {
				if self.id == agent_id {
					MessageRole::Assistant
				} else {
					MessageRole::User
				}
			}
		}
	}

	pub fn items(&self) -> &[ItemId] { &self.items }
}

/// Id associated with an [`Actor`].
/// Actors are components and already have an associated [`Entity`],
/// but we need something more easily handled by distributed systems,
/// databases etc. See also [`ItemId`]
#[derive(
	Debug,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Serialize,
	Deserialize,
)]
pub struct ActorId(Uuid);

impl std::fmt::Display for ActorId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

impl Default for ActorId {
	fn default() -> Self { Self(Uuid::now_v7()) }
}
