use std::path::Path;

use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;


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
	pub fn add_member<'a>(
		&'a mut self,
		bundle: impl Bundle,
	) -> MemberBuilder<'a, 'a> {
		MemberBuilder::new(self.commands.reborrow(), self.session, bundle)
	}
}
pub struct MemberBuilder<'w, 's> {
	pub session: Entity,
	pub member: Entity,
	pub commands: Commands<'w, 's>,
}

impl<'w, 's> MemberBuilder<'w, 's> {
	pub fn new(
		mut commands: Commands<'w, 's>,
		session: Entity,
		bundle: impl Bundle,
	) -> Self {
		let member = commands.spawn((bundle, SessionMemberOf(session))).id();

		Self {
			commands,
			session,
			member,
		}
	}
	pub fn trigger(&mut self, ev: impl Event) -> &mut Self {
		self.commands.entity(self.member).trigger(ev);
		self
	}
	pub fn create_message<'a>(&'a mut self) -> MessageBuilder<'a, 'a> {
		MessageBuilder::new(self.commands.reborrow(), self.session, self.member)
	}
}
pub struct MessageBuilder<'w, 's> {
	message: Entity,
	commands: Commands<'w, 's>,
}

impl<'w, 's> MessageBuilder<'w, 's> {
	pub fn new(
		mut commands: Commands<'w, 's>,
		session: Entity,
		member: Entity,
	) -> Self {
		let message = commands
			.spawn((ChildOf(session), MessageOwner(member)))
			.id();

		Self { commands, message }
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
			OnSpawn::new(|entity| {
				entity.trigger(ContentEnded);
			}),
		));
		self
	}
}
