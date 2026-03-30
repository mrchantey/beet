use crate::prelude::*;
use beet_core::prelude::*;


#[derive(Debug, Clone)]
pub struct ThreadRef<'a> {
	pub entity: Entity,
	pub thread: &'a Thread,
	/// The list of actors in bfs order of [`Children`]
	pub actors: Vec<ActorView<'a>>,
	/// The list of posts in this thread, sorted chronologically by [`PostId`]
	pub posts: Vec<PostView<'a>>,
}

impl std::ops::Deref for ThreadRef<'_> {
	type Target = Thread;
	fn deref(&self) -> &Self::Target { self.thread }
}


impl<'a> ThreadRef<'a> {
	pub fn id(&self) -> ThreadId { self.thread.id() }

	pub fn actor(&self, actor_entity: Entity) -> Result<&ActorView<'a>> {
		self.actors
			.iter()
			.find(|av| av.entity == actor_entity)
			.ok_or_else(|| {
				bevyhow!(
					"No actor for entity {actor_entity} found in thread {thread:?}",
					thread = self.thread
				)
			})
	}
	pub fn actor_from_id(&self, actor_id: ActorId) -> Result<&ActorView<'a>> {
		self.actors
			.iter()
			.find(|av| av.actor.id() == actor_id)
			.ok_or_else(|| {
				bevyhow!(
					"No actor with id {actor_id} found in thread {thread:?}",
					thread = self.thread
				)
			})
	}
	pub fn post_from_id(&self, post_id: PostId) -> Result<&PostView<'a>> {
		self.posts
			.iter()
			.find(|pv| pv.post.id() == post_id)
			.ok_or_else(|| {
				bevyhow!(
					"No post with id {post_id} found in thread {thread:?}",
					thread = self.thread
				)
			})
	}

	/// Find a stored [`ResponseMeta`] for the given actor, provider, and model.
	pub fn stored_response(
		&self,
		actor: Entity,
		provider_slug: &str,
		model_slug: &str,
	) -> Option<(&PostView<'_>, &ResponseMeta)> {
		self.posts.iter().find_map(|pv| {
			if pv.actor_entity != actor {
				return None;
			}
			let meta = pv.response_meta?;
			(meta.provider_slug == provider_slug
				&& meta.model_slug == model_slug
				&& meta.response_stored)
				.then_some((pv, meta))
		})
	}

	pub fn posts_from(&self, after_post: Option<PostId>) -> Vec<PostView<'_>> {
		if let Some(after) = after_post {
			match self.posts.iter().position(|a| a.id() == after) {
				Some(i) => self.posts[i + 1..].to_vec(),
				None => self.posts.clone(),
			}
		} else {
			self.posts.clone()
		}
	}
}

#[derive(Debug, Clone)]
pub struct ActorView<'a> {
	pub entity: Entity,
	pub actor: &'a Actor,
	pub tool_choice: Option<&'a ToolChoice>,
}

impl std::ops::Deref for ActorView<'_> {
	type Target = Actor;
	fn deref(&self) -> &Self::Target { self.actor }
}

#[derive(Debug, Clone)]
pub struct PostView<'a> {
	pub entity: Entity,
	pub actor_entity: Entity,
	pub post: &'a Post,
	pub actor: &'a Actor,
	pub response_meta: Option<&'a ResponseMeta>,
}


impl std::ops::Deref for PostView<'_> {
	type Target = Post;
	fn deref(&self) -> &Self::Target { self.post }
}

impl PostView<'_> {
	pub fn entity(&self) -> Entity { self.entity }
	pub fn actor_id(&self) -> ActorId { self.actor.id() }

	/// Wraps text in XML metadata tags so models can distinguish speakers.
	/// Used for non-assistant, non-system, non-developer messages.
	pub fn wrap_user_text(&self, text: &str) -> String {
		format!(
			"<post author={} author_kind={} author_id={}>{}</post>",
			self.actor.name(),
			self.actor.kind().input_str(),
			self.actor.id(),
			text
		)
	}
}
