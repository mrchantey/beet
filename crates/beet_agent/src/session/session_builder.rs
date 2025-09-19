use std::path::Path;

use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;


/// Helper for building sessions, using user defined ids to match actors with content.
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
	pub fn add_actor<'a>(
		&'a mut self,
		bundle: impl Bundle,
	) -> ActorBuilder<'a, 'a> {
		ActorBuilder::new(self.commands.reborrow(), self.session, bundle)
	}
}
pub struct ActorBuilder<'w, 's> {
	pub session: Entity,
	pub actor: Entity,
	pub commands: Commands<'w, 's>,
}

impl<'w, 's> ActorBuilder<'w, 's> {
	pub fn new(
		mut commands: Commands<'w, 's>,
		session: Entity,
		bundle: impl Bundle,
	) -> Self {
		let actor = commands.spawn((bundle, ChildOf(session))).id();

		Self {
			commands,
			session,
			actor,
		}
	}
	pub fn trigger(&mut self, ev: impl Event) -> &mut Self {
		self.commands.entity(self.actor).trigger(ev);
		self
	}
	pub fn create_message<'a>(&'a mut self) -> MessageBuilder<'a, 'a> {
		MessageBuilder::new(self.commands.reborrow(), self.session, self.actor)
	}
}
pub struct MessageBuilder<'w, 's> {
	pub session: Entity,
	pub actor: Entity,
	pub message: Entity,
	pub commands: Commands<'w, 's>,
}

impl<'w, 's> MessageBuilder<'w, 's> {
	pub fn new(
		mut commands: Commands<'w, 's>,
		session: Entity,
		actor: Entity,
	) -> Self {
		let message = commands.spawn((ChildOf(actor), Message::default())).id();
		Self {
			commands,
			session,
			actor,
			message,
		}
	}
	pub fn add_text(&mut self, text: impl AsRef<str>) -> &mut Self {
		self.add_content(TextContent::new(text))
	}
	pub async fn add_workspace_file(
		&mut self,
		path: impl AsRef<Path>,
	) -> Result<&mut Self> {
		self.add_file(AbsPathBuf::new_workspace_rel(path).unwrap().to_string())
			.await
	}

	pub async fn add_file(
		&mut self,
		path: impl AsRef<str>,
	) -> Result<&mut Self> {
		let file = FileContent::new(path).await?;
		self.add_content(file).xok()
	}
	pub fn add_content(&mut self, content: impl Bundle) -> &mut Self {
		self.commands.spawn((
			ChildOf(self.message),
			content,
			ContentComplete::default(),
		));
		self
	}
}
