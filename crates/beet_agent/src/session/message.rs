use crate::prelude::*;
use beet_core::prelude::*;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use std::fmt::Debug;
use std::hash::Hash;



pub struct MessageView<'a> {
	pub actor: ActorView<'a>,
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
pub struct MessageRequest;



/// Marker component indicating the root entity for an actor's message.
/// Messages must be (possibly nested) descendents of an [`Actor`], and may
/// contain Content either in its entity its descendents.
#[derive(Debug, Clone, Component)]
pub struct Message {
	pub created: Instant,
}

impl Default for Message {
	fn default() -> Self {
		Self {
			created: Instant::now(),
		}
	}
}


/// Added to a [`Message`] when it is finished, and no more content
/// will be added to it.
#[derive(Debug, Clone, Component)]
pub struct MessageComplete;


pub struct MessageSpawner<T: Hash> {
	pub queue: AsyncQueue,
	pub message: Entity,
	pub content_map: HashMap<T, Entity>,
	pub finished_content: Vec<T>,
}
impl<T: Hash + Eq + Debug> MessageSpawner<T> {
	pub async fn spawn(queue: AsyncQueue, actor: Entity) -> Result<Self> {
		let message =
			queue.spawn_then((Message::default(), ChildOf(actor))).await;


		Self {
			queue,
			message,
			content_map: HashMap::default(),
			finished_content: Vec::new(),
		}
		.xok()
	}
	pub async fn add(
		&mut self,
		key: T,
		content: impl Bundle,
	) -> Result<&mut Self> {
		if self.content_map.contains_key(&key) {
			bevybail!("Duplicate output index: {key:?}");
		} else {
			let entity = self
				.queue
				.spawn_then((ChildOf(self.message), content))
				.await;
			self.content_map.insert(key, entity);
		}
		Ok(self)
	}

	/// Inserts the bundle, creating the entity if it doesnt exist
	pub async fn insert(
		&mut self,
		key: T,
		bundle: impl Bundle,
	) -> Result<&mut Self> {
		if let Ok(entity) = self.get_entity(&key) {
			self.queue.entity(entity).insert(bundle).await;
		} else {
			self.add(key, bundle).await?;
		}
		Ok(self)
	}

	fn get_entity(&self, key: &T) -> Result<Entity> {
		self.content_map
			.get(key)
			.copied()
			.ok_or_else(|| bevyhow!("Missing entity for index: {key:?}"))
	}

	pub async fn add_or_delta(
		&mut self,
		key: T,
		text: impl AsRef<str>,
	) -> Result<&mut Self> {
		if self.content_map.contains_key(&key) {
			self.text_delta(key, text).await
		} else {
			self.add(key, TextContent::new(text)).await
		}
	}

	pub async fn text_delta(
		&mut self,
		key: T,
		text: impl AsRef<str>,
	) -> Result<&mut Self> {
		self.queue
			.entity(self.get_entity(&key)?)
			.trigger(TextDelta::new(text))
			.await;
		Ok(self)
	}

	pub async fn finish_content(&mut self, key: T) -> Result<&mut Self> {
		self.queue
			.entity(self.get_entity(&key)?)
			.insert(ContentEnded::default())
			.await;
		self.finished_content.push(key);
		Ok(self)
	}
	pub async fn finish_message(&mut self) -> Result {
		let unfinished = self
			.content_map
			.iter()
			.filter(|(k, _)| !self.finished_content.contains(k))
			.map(|(_, v)| *v)
			.collect::<Vec<_>>();

		let message = self.message;
		self.queue
			.with_then(move |world| {
				for entity in unfinished {
					world.entity_mut(entity).insert(ContentEnded::default());
				}
				world.entity_mut(message).insert(MessageComplete);
			})
			.await;
		Ok(())
	}
}
