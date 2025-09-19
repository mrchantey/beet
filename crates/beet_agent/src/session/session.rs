use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

pub struct AgentPlugin;

impl Plugin for AgentPlugin {
	fn build(&self, app: &mut App) {
		app.add_observer(broadcast_content_event::<MessageStart>)
			.add_observer(broadcast_content_event::<TextDelta>);
	}
}

/// When any content changes, notify the session and all actors who
/// do not own the content
fn broadcast_content_event<E: Clone + Event>(
	trigger: Trigger<E>,
	mut commands: Commands,
	cx: SessionQuery,
) -> Result {
	let emitter = trigger.target();
	let session = cx.session(emitter)?;
	let actor = cx.actor(emitter)?;
	let message = cx.message(emitter)?;
	let targets = cx.actors(session)?.xpush(session);
	commands.trigger_targets(
		ContentBroadcast {
			message,
			actor,
			session,
			event: trigger.event().clone(),
		},
		targets,
	);

	Ok(())
}

#[derive(Default, Component)]
pub struct Session;

/// Actor representing an AI agent, more than one agent may
/// participate in a session at a time
#[derive(Default, Component)]
#[require(Actor, TokenUsage, Name = Name::new("Agent"))]
pub struct Agent;

/// Actor representing a human using the program, more than
/// one user may participate in a session at a time.
#[derive(Component)]
#[require(Actor, Name = Name::new("User"))]
pub struct User;

/// Also known as the system actor, the messages and actions
/// performed by this entity carry more weight.
/// For example [`Developer`] instructions overrule [`User`] instructions
#[derive(Component)]
#[require(Actor, Name = Name::new("Developer"))]
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


/// Helper for getting and setting session info
#[derive(SystemParam)]
pub struct SessionQuery<'w, 's> {
	children: Query<'w, 's, &'static Children>,
	parents: Query<'w, 's, &'static ChildOf>,
	sessions: Query<'w, 's, Entity, With<Session>>,
	actors: Query<
		'w,
		's,
		(
			Entity,
			&'static Name,
			Option<&'static User>,
			Option<&'static Agent>,
			Option<&'static Developer>,
		),
		With<Actor>,
	>,
	messages: Query<'w, 's, &'static Message>,
	content: Query<
		'w,
		's,
		(Option<&'static TextContent>, Option<&'static FileContent>),
		Or<(With<TextContent>, With<FileContent>)>,
	>,
}

impl SessionQuery<'_, '_> {
	/// Get the session for this entity
	pub fn session(&self, entity: Entity) -> Result<Entity> {
		self.parents
			.iter_ancestors_inclusive(entity)
			.find(|ent| self.sessions.get(*ent).is_ok())
			.ok_or_else(|| bevyhow!("no session found for entity {entity:?}"))
	}

	/// Get the nearest message ancestor for this entity
	pub fn message(&self, entity: Entity) -> Result<Entity> {
		self.parents
			.iter_ancestors_inclusive(entity)
			.find(|ent| self.messages.get(*ent).is_ok())
			.ok_or_else(|| bevyhow!("no message found for entity {entity:?}"))
	}
	/// Get the nearest actor ancestor for this entity
	pub fn actor(&self, entity: Entity) -> Result<Entity> {
		self.parents
			.iter_ancestors_inclusive(entity)
			.find(|ent| self.actors.get(*ent).is_ok())
			.ok_or_else(|| bevyhow!("no actor found for entity {entity:?}"))
	}
	pub fn actors(&self, entity: Entity) -> Result<Vec<Entity>> {
		let session = self.session(entity)?;
		self.children
			.iter_direct_descendants(session)
			.filter(|ent| self.actors.get(*ent).is_ok())
			.collect::<Vec<_>>()
			.xok()
	}

	pub fn collect_messages(
		&self,
		actor: Entity,
	) -> Result<Vec<MessageView<'_>>> {
		let session = self.session(actor)?;

		let actors = self.children.iter_direct_descendants(session).filter_map(
			|entity| {
				let (entity, name, user, agent, developer) =
					self.actors.get(entity).ok()?;
				let role = if user.is_some() {
					Some(ActorRole::User)
				} else if agent.is_some() {
					Some(ActorRole::Agent)
				} else if developer.is_some() {
					Some(ActorRole::Developer)
				} else {
					None
				};
				Some((entity, name, role))
			},
		);

		let mut messages = actors
			.flat_map(|(actor_ent, _name, role)| {
				self.children
					.iter_descendants_depth_first(actor_ent)
					.filter_map(move |msg_ent| {
						self.messages
							.get(msg_ent)
							.map(move |message| {
								(actor_ent, role, msg_ent, message)
							})
							.ok()
					})
			})
			.map(|(actor_ent, role, msg_ent, message)| {
				let rel_role = if actor_ent == actor {
					RelativeRole::This
				} else if role == Some(ActorRole::Developer) {
					RelativeRole::Developer
				} else {
					RelativeRole::Other
				};
				let mut parts = Vec::new();
				self.get_content_recursive(msg_ent, &mut parts);
				MessageView {
					message,
					content: parts,
					role: rel_role,
				}
			})
			.collect::<Vec<_>>();
		messages.sort_by_key(|mv| mv.message.created);
		messages.xok()
	}

	/// recursively get all content from this item
	fn get_content_recursive<'a>(
		&'a self,
		parent: Entity,
		content: &mut Vec<ContentView<'a>>,
	) {
		if let Ok(items) = self.content.get(parent) {
			match items {
				(Some(text), None) => content.push(ContentView::Text(text)),
				(None, Some(file)) => content.push(ContentView::File(file)),
				_ => unreachable!("content must be text or file"),
			}
		}

		self.children
			.iter_direct_descendants(parent)
			.for_each(|entity| {
				self.get_content_recursive(entity, content);
			});
	}
}
pub enum ContentView<'a> {
	Text(&'a TextContent),
	File(&'a FileContent),
}
impl ContentView<'_> {
	pub fn as_text(&self) -> Option<&TextContent> {
		match self {
			ContentView::Text(text) => Some(text),
			_ => None,
		}
	}
	pub fn as_file(&self) -> Option<&FileContent> {
		match self {
			ContentView::File(file) => Some(file),
			_ => None,
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorRole {
	User,
	Developer,
	Agent,
}


pub struct MessageView<'a> {
	pub role: RelativeRole,
	pub message: &'a Message,
	pub content: Vec<ContentView<'a>>,
}

/// A role relative to the actor:
/// -  If the actor owns the content the role is [`Role::This`],
/// - otherwise if the owner has a [`Developer`] component the role is [`Role::Developer`].
/// Any other case is [`Role::Other`] which may be a user or another agent
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelativeRole {
	/// The role is this actor,
	This,
	Developer,
	Other,
}

impl RelativeRole {}

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
		assertion: impl 'static + Send + Sync + Fn(Vec<ContentView>),
	) {
		dotenv::dotenv().ok();

		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AsyncPlugin, AgentPlugin));

		let mut session = SessionBuilder::from_app(&mut app);
		let mut user = session.add_actor(User);
		message(user.create_message()).await;
		session.add_actor(agent).trigger(StartResponse);

		app.add_observer(
			move |ev: Trigger<ResponseComplete>,
			      mut commands: Commands,
			      cx: SessionQuery| {
				let actor = ev.target();
				let content = cx
					.collect_messages(actor)
					.unwrap()
					.into_iter()
					.find(|msg| msg.role == RelativeRole::This)
					.unwrap()
					.content;

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
				content[0].as_text().unwrap().0.xref().xpect_contains("6");
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
				content[0]
					.as_text()
					.unwrap()
					.0
					.xref()
					.xpect_contains("pineapple");
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
					.as_text()
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
				msg.add_text("create a logo for beet, a metaframework for the bevy engine. the logo should be of a beetroot, with clean lines that can scale down to a tiny favicon");
			},
			|content| {
				use base64::prelude::*;
				let file =
					content.iter().find_map(|item| item.as_file()).unwrap();
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
