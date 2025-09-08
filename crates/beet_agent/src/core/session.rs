use beet_core::bevyhow;
use bevy::ecs::relationship::Relationship;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use std::collections::HashMap;


pub struct AgentPlugin;

impl Plugin for AgentPlugin {
	fn build(&self, app: &mut App) { app.add_systems(Update, content_changed); }
}

/// When any content changes, notify all session members
fn content_changed(
	content: Populated<(Entity, &ChildOf, &ContentOwner), Changed<TextContent>>,
	mut commands: Commands,
	sessions: Query<&SessionMembers>,
) -> Result {
	for (entity, parent, owner) in content.iter() {
		let members = sessions.get(parent.parent())?.iter().collect::<Vec<_>>();
		commands.trigger_targets(
			ContentChanged {
				session: parent.parent(),
				owner: owner.get(),
				content: entity,
			},
			members,
		);
	}
	Ok(())
}

/// Point to the [`Session`] this member is a part of
#[derive(Deref, Component)]
#[relationship(relationship_target = SessionMembers)]
pub struct SessionMemberOf(pub Entity);

/// All user, agent and developer entities participating in
/// this session.
#[derive(Deref, Component)]
#[relationship_target(relationship = SessionMemberOf)]
#[require(Session)]
pub struct SessionMembers(Vec<Entity>);

#[derive(Default, Component)]
pub struct Session;

/// Helper for building sessions, using user defined ids to match members with content.
#[derive(Default)]
pub struct SessionBuilder {
	members: HashMap<usize, Box<dyn FnOnce(&mut Commands, Entity) -> Entity>>,
	content: Vec<
		Box<
			dyn FnOnce(&mut Commands, Entity, HashMap<usize, Entity>) -> Result,
		>,
	>,
}
impl SessionBuilder {
	pub fn new() -> Self { Self::default() }
	pub fn member(mut self, id: usize, bundle: impl Bundle) -> Self {
		self.members.insert(
			id,
			Box::new(move |cmd, session| {
				cmd.spawn((bundle, SessionMemberOf(session))).id()
			}),
		);
		self
	}

	pub fn content(
		mut self,
		id: usize,
		text: impl 'static + AsRef<str>,
	) -> Self {
		self.content.push(Box::new(move |cmd, session, members| {
			let member = members
				.get(&id)
				.ok_or_else(|| bevyhow!("Member with id {id} not found"))?;

			cmd.spawn((
				ChildOf(session),
				ContentOwner(*member),
				TextContent::new(text),
			));
			Ok(())
		}));
		self
	}
	pub fn build(mut self, commands: &mut Commands) -> Result {
		let session = commands.spawn(Session).id();
		let members = self
			.members
			.drain()
			.map(|(id, member)| {
				let entity = member(commands, session);
				(id, entity)
			})
			.collect::<HashMap<_, _>>();
		self.content
			.into_iter()
			.map(|content| content(commands, session, members.clone()))
			.collect::<Result<Vec<_>>>()?;
		Ok(())
	}
}

/// Point to the owner of this content.
#[derive(Deref, Component)]
#[relationship(relationship_target = OwnedContent)]
pub struct ContentOwner(pub Entity);

/// List of content owned by the developer, user, or agent.
/// This is non-linked so the owner may be removed but the content
/// remains, ie somebody leaving a chat session.
#[derive(Deref, Component)]
#[relationship_target(relationship = ContentOwner)]
pub struct OwnedContent(Vec<Entity>);


/// Event notifying session members the content has changed
#[derive(Event)]
pub struct ContentChanged {
	/// The [`ContentOwner`] of the changed content
	pub owner: Entity,
	/// The [`Session`] which the changed content belongs to
	pub session: Entity,
	/// The entity containing the changed [`Content`]
	pub content: Entity,
}
#[derive(Event)]
pub struct ResponseComplete;

#[derive(Default, Component)]
#[require(TokenUsage)]
pub struct Agent;
/// Also known as the system role, this entity represents content
/// owned by the developer, thats you :)
#[derive(Component)]
pub struct Developer;

#[derive(Debug, Default, Component)]
pub struct TokenUsage {
	pub input_tokens: u64,
	pub output_tokens: u64,
}

pub enum ReasoningEffort {
	Min,
	Max,
}

#[derive(Default, Deref, DerefMut, Component)]
pub struct TextContent(pub String);

impl TextContent {
	pub fn new(text: impl AsRef<str>) -> Self {
		TextContent(text.as_ref().to_string())
	}
}

#[derive(SystemParam)]
pub struct SessionQuery<'w, 's> {
	children: Query<'w, 's, &'static Children>,
	content: Query<'w, 's, (&'static ContentOwner, &'static TextContent)>,
	developers: Query<'w, 's, &'static Developer>,
}

impl SessionQuery<'_, '_> {
	pub fn content_changed(
		&self,
		trigger: &Trigger<ContentChanged>,
	) -> Vec<ContentItem<'_>> {
		let member = trigger.target();
		self.children
			.iter_descendants_depth_first(trigger.session)
			.filter_map(|entity| self.content.get(entity).ok())
			.map(|(owner, text)| {
				let role = if **owner == member {
					Role::This
				} else if self.developers.get(**owner).is_ok() {
					Role::Developer
				} else {
					Role::Other
				};
				ContentItem {
					content: Content::Text(&text),
					role,
				}
			})
			.collect()
	}
}
pub enum Content<'a> {
	Text(&'a TextContent),
}

pub struct ContentItem<'a> {
	pub content: Content<'a>,
	pub role: Role,
}

/// A role relative to the member:
/// -  If the member owns the content the role is [`Role::This`],
/// - otherwise if the owner has a [`Developer`] component the role is [`Role::Developer`].
/// Any other case is [`Role::Other`] which may be a user or another agent
#[derive(Debug, Clone)]
pub enum Role {
	/// The role is this member,
	This,
	Developer,
	Other,
}

impl Role {}

/// An id used by a Content Provider to associate this entity and
/// its children with a particular response.
pub struct ResponseId {
	pub id: String,
}

/// The provider id for a particular piece of content, for example
/// the OpenAi `response.message.id` which may be streamed.
pub struct ContentId {
	pub id: String,
}


/// Text emitted to stateless outputs like stdout or TTS
#[derive(Event)]
pub struct TextDelta(pub String);


// #[cfg(test)]
// mod test {
// 	use crate::prelude::*;
// 	use sweet::prelude::*;



// }


#[cfg(test)]
mod test {
	// use crate::prelude::*;
	// use sweet::prelude::*;

	#[test]
	fn works() {}
}
