use crate::prelude::*;
use beet_core::prelude::*;
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
	content: Populated<(Entity, &ChildOf)>,
	mut commands: Commands,
	messages: Query<(&ChildOf, &MessageOwner)>,
	sessions: Query<&SessionMembers>,
) -> Result {
	let (content, parent) = content.get(trigger.target())?;
	let (session, owner) = messages.get(parent.parent())?;
	let session = session.parent();
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
	messages: Query<'w, 's, (Entity, &'static MessageOwner)>,
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

/// Text emitted to stateless outputs like stdout or TTS
#[derive(Event)]
pub struct TextDelta(pub String);

/// Indicate it is 'your turn'
#[derive(Event)]
pub struct StartResponse;

#[derive(Event)]
pub struct ResponseComplete;

#[cfg(test)]
pub(super) mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;


	async fn run_assertion(
		agent: impl Bundle,
		message: impl AsyncFnOnce(MessageBuilder),
		assertion: impl 'static
		+ Send
		+ Sync
		+ Fn(Vec<(Option<&TextContent>, Option<&FileContent>)>),
	) {
		dotenv::dotenv().ok();

		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AsyncPlugin, AgentPlugin));

		let mut session = SessionBuilder::from_app(&mut app);
		let mut user = session.add_member(User);
		message(user.create_message()).await;
		session.add_member(agent).trigger(StartResponse);

		app.add_observer(
			move |ev: Trigger<ResponseComplete>,
			      mut commands: Commands,
			      content: Query<
				(Option<&TextContent>, Option<&FileContent>),
				Or<(With<TextContent>, With<FileContent>)>,
			>,
			      children: Query<&Children>,
			      query: Query<(&TokenUsage, &OwnedMessages)>| {
				let (_tokens, messages) = query.get(ev.target()).unwrap();
				let content = children
					.get(messages[0])
					.unwrap()
					.iter()
					.filter_map(|ent| content.get(ent).ok())
					.collect::<Vec<_>>();
				assertion(content);
				// let text = text.get(content[0]).unwrap().0.xref();
				// println!("Agent > {}\n", text);
				commands.send_event(AppExit::Success);
			},
		);

		app.run_async(AsyncChannel::runner_async)
			.await
			.into_result()
			.unwrap();
	}

	pub async fn text_to_text(agent: impl Bundle) {
		run_assertion(
			agent,
			async |mut msg| {
				msg.add_text("what is 2 + 4");
			},
			|content| {
				content[0].0.unwrap().0.xref().xpect_contains("6");
			},
		)
		.await;
	}
	pub async fn textfile_to_text(agent: impl Bundle) {
		run_assertion(
			agent,
			async |mut msg| {
				msg.add_text("what is the secret message")
					.add_workspace_file(
						"assets/tests/agents/secret-message.txt",
					)
					.await
					.unwrap();
			},
			|content| {
				content[0].0.unwrap().0.xref().xpect_contains("pineapple");
			},
		)
		.await;
	}

	pub async fn image_to_text(agent: impl Bundle) {
		run_assertion(
			agent,
			async |mut msg| {
				msg.add_text("what does the text in the image say.")
					.add_workspace_file("assets/tests/agents/secret-image.png")
					.await
					.unwrap();
			},
			|content| {
				content[0]
					.0
					.unwrap()
					.0
					.xref()
					.to_lowercase()
					.xpect_contains("bevy");
			},
		)
		.await;
	}
	pub async fn text_to_image(agent: impl Bundle) {
		run_assertion(
			agent,
			async |mut msg| {
				msg.add_text("create an image of a duck");
			},
			|content| {
				use base64::prelude::*;
				let file = content[0].1.unwrap();
				let FileData::Base64(b64) = &file.data else {
					panic!("expected base64 image data");
				};
				let bytes = BASE64_STANDARD.decode(b64).unwrap();
				FsExt::write(
					AbsPathBuf::new_workspace_rel(".cache/file.png").unwrap(),
					bytes,
				)
				.unwrap();
			},
		)
		.await;
	}
}
