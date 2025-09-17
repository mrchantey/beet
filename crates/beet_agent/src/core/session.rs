use beet_core::prelude::EntityObserver;
use beet_core::prelude::HierarchyQueryExtExt;
use beet_core::prelude::OnSpawn;
use beet_core::prelude::Xtend;
use bevy::ecs::component::HookContext;
use bevy::ecs::relationship::Relationship;
use bevy::ecs::system::SystemParam;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;


pub struct AgentPlugin;

impl Plugin for AgentPlugin {
	fn build(&self, app: &mut App) {
		app.add_observer(broadcast_content_event::<ContentAdded>)
			.add_observer(broadcast_content_event::<ContentTextDelta>)
			.add_observer(broadcast_content_event::<ContentEnded>);
	}
}

/// When any content changes, notify the session and all members who
/// do not own the content
fn broadcast_content_event<E: Clone + Event>(
	trigger: Trigger<E>,
	content: Populated<(Entity, &ChildOf, &ContentOwner)>,
	mut commands: Commands,
	sessions: Query<&SessionMembers>,
) -> Result {
	let (content, parent, owner) = content.get(trigger.target())?;
	let session = parent.parent();
	let owner = owner.get();
	let to_notify = sessions
		.iter_descendants_inclusive(session)
		// .filter(|entity| *entity != owner)
		.collect::<Vec<_>>();
	commands.trigger_targets(
		ContentBroadcast {
			content,
			owner,
			session,
			event: trigger.event().clone(),
		},
		to_notify,
	);

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
pub struct SessionBuilder<'w, 's> {
	session: Entity,
	commands: Commands<'w, 's>,
}
impl<'w, 's> SessionBuilder<'w, 's> {
	pub fn session(&self) -> Entity { self.session }
	pub fn commands(&mut self) -> &mut Commands<'w, 's> { &mut self.commands }
	pub fn from_app(app: &'w mut App) -> Self
	where
		'w: 's,
	{
		Self::new(app.world_mut().commands())
	}
	pub fn new(mut commands: Commands<'w, 's>) -> Self {
		let session = commands.spawn(Session).id();
		Self { session, commands }
	}
	pub fn add_member(&mut self, bundle: impl Bundle) -> Entity {
		self.commands
			.spawn((bundle, SessionMemberOf(self.session)))
			.id()
	}
	pub fn add_content(
		&mut self,
		owner: Entity,
		text: impl AsRef<str>,
	) -> &mut Self {
		self.commands.spawn((
			text_content(self.session, owner, text),
			OnSpawn::new(|entity| {
				entity.trigger(ContentEnded);
			}),
		));
		self
	}
}

pub fn text_content(
	session: Entity,
	owner: Entity,
	text: impl AsRef<str>,
) -> impl Bundle {
	(
		ChildOf(session),
		ContentOwner(owner),
		TextContent::new(text),
	)
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


/// Event notifying session members the content has ended
// TODO bevy 0.17 shouldnt need this, we have original entity
#[derive(Clone, Event)]
pub struct ContentBroadcast<E> {
	pub content: Entity,
	pub session: Entity,
	pub owner: Entity,
	pub event: E,
}



/// Emitted on a piece of content like a TextContent to indicate it has started.
/// This event does not contain text.
#[derive(Clone, Event)]
pub struct ContentAdded;
/// Emitted on a piece of content like a TextContent to indicate a new piece of text
/// was added.
#[derive(Clone, Event)]
pub struct ContentTextDelta(pub String);


impl ContentTextDelta {
	pub fn new(text: impl AsRef<str>) -> Self {
		Self(text.as_ref().to_string())
	}
}
/// Emitted on a piece of content like a TextContent to indicate it has finished
/// streaming.
#[derive(Clone, Event)]
pub struct ContentEnded;

#[derive(Event)]
pub struct ResponseComplete;

/// A content owner controlled by an AI agent, more than one agent may
/// exist at a time
#[derive(Default, Component)]
#[require(TokenUsage)]
pub struct Agent;

/// A content owner controlled by a person using the program, more than
/// one user may exist at a time
#[derive(Component)]
pub struct User;

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
#[component(on_add=text_delta_observer)]
pub struct TextContent(pub String);

fn text_delta_observer(mut world: DeferredWorld, cx: HookContext) {
	let initial_text = world
		.entity(cx.entity)
		.get::<TextContent>()
		.unwrap()
		.0
		.clone();
	let mut commands = world.commands();
	let mut entity = commands.entity(cx.entity);

	entity.trigger(ContentAdded);
	if !initial_text.is_empty() {
		entity.trigger(ContentTextDelta::new(initial_text));
	}
	entity.insert(EntityObserver::new(
		|delta: Trigger<ContentTextDelta>,
		 mut text_content: Query<&mut TextContent>|
		 -> Result {
			text_content.get_mut(delta.target())?.0.push_str(&delta.0);
			Ok(())
		},
	));
}
impl TextContent {
	pub fn new(text: impl AsRef<str>) -> Self {
		TextContent(text.as_ref().to_string())
	}
}


#[derive(SystemParam)]
pub struct SessionContext<'w, 's> {
	children: Query<'w, 's, &'static Children>,
	content: Query<'w, 's, (&'static ContentOwner, &'static TextContent)>,
	developers: Query<'w, 's, &'static Developer>,
}

impl SessionContext<'_, '_> {
	pub fn collect_content_relative(
		&self,
		session: Entity,
		member: Entity,
	) -> Result<Vec<ContentItem<'_>>> {
		self.children
			.iter_descendants_depth_first(session)
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
			.collect::<Vec<_>>()
			.xok()
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
