use beet_core::prelude::*;


#[derive(Debug, Clone)]
pub struct ActorView<'a> {
	pub entity: Entity,
	pub name: &'a Name,
	pub role: ActorRole,
}
impl<'a> ActorView<'a> {
	pub fn new(
		(entity, name, role): (Entity, &'a Name, &'a ActorRole),
	) -> Self {
		Self {
			entity,
			name,
			role: *role,
		}
	}
}

/// A session actor, owns messages and reacts to others
#[derive(Debug, Clone, Copy, PartialEq, Eq, Component, strum::Display)]
pub enum ActorRole {
	User,
	Developer,
	Agent,
}

/// Actor representing an AI agent, more than one agent may
/// participate in a session at a time
#[derive(Default, Component)]
#[require(
	TokenUsage,
	Name = Name::new("Agent"),
	ActorRole=ActorRole::Agent
)]
pub struct AgentRole;

/// Actor representing a human using the program, more than
/// one user may participate in a session at a time.
#[derive(Component)]
#[require(
	Name = Name::new("User"),
	ActorRole=ActorRole::User
)]
pub struct UserRole;

/// Also known as the system actor, the messages and actions
/// performed by this entity carry more weight.
/// For example [`DeveloperRole`] instructions overrule [`UserRole`] instructions
#[derive(Component)]
#[require(
	Name = Name::new("User"),
	ActorRole=ActorRole::Developer
)]
pub struct DeveloperRole;



#[derive(Debug, Default, Component)]
pub struct TokenUsage {
	pub input_tokens: u64,
	pub output_tokens: u64,
}

pub enum ReasoningEffort {
	Min,
	Max,
}
