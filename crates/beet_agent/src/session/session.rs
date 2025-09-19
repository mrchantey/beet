use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

pub struct AgentPlugin;

impl Plugin for AgentPlugin {
	fn build(&self, _: &mut App) {}
}

#[derive(Default, Component)]
pub struct Session;


/// Helper for getting and setting session info
#[derive(SystemParam)]
pub struct SessionParams<'w, 's> {
	children: Query<'w, 's, &'static Children>,
	parents: Query<'w, 's, &'static ChildOf>,
	sessions: Query<'w, 's, Entity, With<Session>>,
	actors: Query<'w, 's, (Entity, &'static Name, &'static ActorRole)>,
	messages: Query<'w, 's, &'static Message>,
	content: Query<
		'w,
		's,
		(Option<&'static TextContent>, Option<&'static FileContent>),
		Or<(With<TextContent>, With<FileContent>)>,
	>,
}

impl SessionParams<'_, '_> {
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
	pub fn actor<'a>(&'a self, entity: Entity) -> Result<ActorView<'a>> {
		self.parents
			.iter_ancestors_inclusive(entity)
			.find_map(|ent| self.actors.get(ent).map(ActorView::new).ok())
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
			|entity| self.actors.get(entity).ok().map(ActorView::new),
		);

		let mut messages = actors
			.flat_map(|actor| {
				self.children
					.iter_descendants_depth_first(actor.entity)
					.filter_map(move |msg_ent| {
						let actor = actor.clone();
						self.messages
							.get(msg_ent)
							.map(move |message| {
								let mut parts = Vec::new();
								self.get_content_recursive(msg_ent, &mut parts);
								MessageView {
									actor: actor.clone(),
									message,
									content: parts,
								}
							})
							.ok()
					})
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
		if let Ok(query) = self.content.get(parent) {
			match query {
				(Some(text), None) => content.push(ContentView::Text(text)),
				(None, Some(file)) => content.push(ContentView::File(file)),
				_ => {
					// should be unreachable?
				} // _ => unreachable!("content must be text or file"),
			}
		}

		self.children
			.iter_direct_descendants(parent)
			.for_each(|entity| {
				self.get_content_recursive(entity, content);
			});
	}
}

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
		session.add_actor(agent).trigger(MessageRequest);

		app.add_observer(
			move |ev: Trigger<OnAdd, MessageComplete>,
			      mut commands: Commands,
			      cx: SessionParams|
			      -> Result {
				let actor = cx.actor(ev.target())?;
				if actor.role != ActorRole::Agent {
					return Ok(());
				};
				let content = cx
					.collect_messages(actor.entity)
					.unwrap()
					.into_iter()
					.find(|msg| msg.actor.entity == actor.entity)
					.unwrap()
					.content;

				assertion(content);
				// let text = text.get(content[0]).unwrap().0.xref();
				// println!("Agent > {}\n", text);
				commands.send_event(AppExit::Success);
				Ok(())
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
				// println!("content: {content:#?}");

				use base64::prelude::*;
				let file =
					content.iter().find_map(|item| item.as_file()).unwrap();
				let FileData::Base64(b64) = &file.data else {
					panic!("expected base64 image data");
				};
				let bytes = BASE64_STANDARD.decode(b64).unwrap();
				fs_ext::write(
					AbsPathBuf::new_workspace_rel(".cache/file.png").unwrap(),
					bytes,
				)
				.unwrap();
			},
		)
		.await;
	}
}
