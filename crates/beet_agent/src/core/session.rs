use crate::prelude::*;
use beet_core::prelude::HierarchyQueryExtExt;
use beet_core::prelude::OnSpawn;
use beet_core::prelude::Xtend;
use bevy::ecs::relationship::Relationship;
use bevy::ecs::system::SystemParam;
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
	// pub fn add_message(
	// 	&mut self,
	// 	owner: Entity,
	// 	content: impl Bundle,
	// ) -> &mut Self {
	// 	self.commands.spawn((
	// 		content_bundle(self.session, owner, content),
	// 		OnSpawn::new(|entity| {
	// 			entity.trigger(ContentEnded);
	// 		}),
	// 	));
	// 	self
	// }
	pub fn add_content(
		&mut self,
		owner: Entity,
		content: impl Bundle,
	) -> &mut Self {
		self.commands.spawn((
			content_bundle(self.session, owner, content),
			OnSpawn::new(|entity| {
				entity.trigger(ContentEnded);
			}),
		));
		self
	}
}


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


/// Helper for getting and setting session context
#[derive(SystemParam)]
pub struct SessionContext<'w, 's> {
	children: Query<'w, 's, &'static Children>,
	messages: Query<'w, 's, (Entity, &'static ContentOwner)>,
	content: Query<
		'w,
		's,
		(Option<&'static TextContent>, Option<&'static FileContent>),
		Or<(With<TextContent>, With<FileContent>)>,
	>,
	developers: Query<'w, 's, &'static Developer>,
}

impl SessionContext<'_, '_> {
	pub fn collect_content_relative(
		&self,
		session: Entity,
		member: Entity,
	) -> Result<Vec<Message<'_>>> {
		self.children
			.iter_descendants_depth_first(session)
			.filter_map(|entity| self.messages.get(entity).ok())
			.map(|(entity, owner)| {
				let role = if **owner == member {
					Role::This
				} else if self.developers.get(**owner).is_ok() {
					Role::Developer
				} else {
					Role::Other
				};
				let mut parts = Vec::new();
				self.get_parts(entity, &mut parts);
				Message { parts, role }
			})
			.collect::<Vec<_>>()
			.xok()
	}

	/// recursively get all content from this item
	fn get_parts<'a>(&'a self, parent: Entity, content: &mut Vec<Content<'a>>) {
		if let Ok(items) = self.content.get(parent) {
			match items {
				(Some(text), None) => content.push(Content::Text(text)),
				(None, Some(file)) => content.push(Content::File(file)),
				_ => unreachable!("content must be text or file"),
			}
		}

		self.children
			.iter_direct_descendants(parent)
			.for_each(|entity| {
				self.get_parts(entity, content);
			});
	}
}
pub enum Content<'a> {
	Text(&'a TextContent),
	File(&'a FileContent),
}

pub struct Message<'a> {
	pub role: Role,
	pub parts: Vec<Content<'a>>,
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
pub(super) mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	pub async fn text_to_text(agent: impl Bundle) {
		dotenv::dotenv().ok();

		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AsyncPlugin, AgentPlugin));

		let mut session = SessionBuilder::from_app(&mut app);
		let user = session.add_member(User);
		let _agent = session.add_member(agent);
		session.add_content(user, TextContent::new("what is 2 + 4?"));

		app.add_observer(
			|ev: Trigger<ResponseComplete>,
			 mut commands: Commands,
			 text: Query<&TextContent>,
			 query: Query<(&TokenUsage, &OwnedContent)>| {
				let (_tokens, content) = query.get(ev.target()).unwrap();
				text.get(content[0]).unwrap().0.xref().xpect_contains("6");
				commands.send_event(AppExit::Success);
			},
		);

		app.run_async(AsyncChannel::runner_async)
			.await
			.into_result()
			.unwrap();
	}
}
