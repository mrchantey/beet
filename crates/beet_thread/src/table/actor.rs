use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// A participant in a thread, a person, an agent or the system, all on equal
/// footing: one row of its thread's `actor` table, keyed by its [`ActorId`].
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	Hash,
	Serialize,
	Deserialize,
	Reflect,
	Component,
)]
#[reflect(Serialize, Deserialize, Component)]
pub struct Actor {
	/// The row's key, never in its body.
	#[serde(skip)]
	id: ActorId,
	/// The name of the actor used when building context
	name: String,
	kind: ActorKind,
}

impl Actor {
	/// An actor whose id is its name in kebab-case ([`ActorId::from_name`]).
	///
	/// ## Errors
	///
	/// Errors when nothing of the name can be an id, asking for one.
	pub fn new(name: impl Into<String>, kind: ActorKind) -> Result<Self> {
		let name = name.into();
		Self::new_with_id(ActorId::from_name(&name)?, name, kind).xok()
	}
	/// An actor with an id of its own, where its name gives none or two
	/// actors share a name.
	pub fn new_with_id(
		id: ActorId,
		name: impl Into<String>,
		kind: ActorKind,
	) -> Self {
		Self {
			id,
			name: name.into(),
			kind,
		}
	}
	pub fn system() -> Self {
		Self::new_with_id(ActorId::SYSTEM, "System", ActorKind::System)
	}
	pub fn developer() -> Self {
		Self::new_with_id(ActorId::DEVELOPER, "Developer", ActorKind::Developer)
	}
	pub fn user() -> Self {
		Self::new_with_id(ActorId::USER, "User", ActorKind::User)
	}
	pub fn agent() -> Self {
		Self::new_with_id(ActorId::AGENT, "Agent", ActorKind::Agent)
	}

	pub fn id(&self) -> &ActorId { &self.id }
	pub fn name(&self) -> &str { &self.name }
	pub fn kind(&self) -> ActorKind { self.kind }
}

impl ActorId {
	/// The id of [`Actor::system`].
	pub const SYSTEM: Self = Self::from_static("system");
	/// The id of [`Actor::developer`].
	pub const DEVELOPER: Self = Self::from_static("developer");
	/// The id of [`Actor::user`].
	pub const USER: Self = Self::from_static("user");
	/// The id of [`Actor::agent`].
	pub const AGENT: Self = Self::from_static("agent");
}

impl TableStoreRow for Actor {
	fn table_name() -> SmolStr { "actor".into() }
	fn key(&self) -> TableKey { self.id.clone().into() }
	fn from_row(key: &TableKey, row: Row) -> Result<Self> {
		let mut actor = row.into_serde::<Self>()?;
		actor.id = ActorId::parse(key)?;
		actor.xok()
	}
}

/// The kind of an actor.
///
/// Distinct from [`MessageRole`](crate::o11s::MessageRole) in that
/// [`ActorKind`] is absolute, whereas the difference between
/// `MessageRole::User` and `MessageRole::Assistant` is relative
/// to the agent reading the post.
#[derive(
	Debug,
	Default,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Serialize,
	Deserialize,
	Reflect,
)]
#[reflect(Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorKind {
	/// Posts by this actor will always be treated as [`MessageRole::System`](crate::o11s::MessageRole::System)
	System,
	/// Posts by this actor will always be treated as [`MessageRole::Developer`](crate::o11s::MessageRole::Developer)
	Developer,
	/// Posts by this actor will always be treated as [`MessageRole::User`](crate::o11s::MessageRole::User)
	User,
	/// Posts by this actor may be treated as [`MessageRole::Assistant`](crate::o11s::MessageRole::Assistant) or [`MessageRole::User`](crate::o11s::MessageRole::User),
	/// depending on which agent is reading. The default kind, ie the
	/// common `<CreateActor>` case.
	#[default]
	Agent,
}

impl std::fmt::Display for ActorKind {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ActorKind::System => write!(f, "system"),
			ActorKind::Developer => write!(f, "developer"),
			ActorKind::User => write!(f, "human"),
			ActorKind::Agent => write!(f, "agent"),
		}
	}
}

impl ActorKind {
	pub fn input_str(&self) -> &'static str {
		match self {
			ActorKind::System => "system",
			ActorKind::Developer => "developer",
			ActorKind::User => "human",
			ActorKind::Agent => "agent",
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn builtin_ids_parse() {
		for id in [
			ActorId::SYSTEM,
			ActorId::DEVELOPER,
			ActorId::USER,
			ActorId::AGENT,
		] {
			ActorId::parse(id.as_str()).unwrap().xpect_eq(id);
		}
	}
}
