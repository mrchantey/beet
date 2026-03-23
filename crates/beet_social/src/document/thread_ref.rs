use crate::prelude::*;
use beet_core::prelude::*;


#[derive(Debug, Clone)]
pub struct ThreadRef<'a> {
	pub entity: Entity,
	pub thread: &'a Thread,
	/// The list of users in bfs order of [`Children`]
	pub users: Vec<UserView<'a>>,
	/// The list of posts in this thread, sorted chronologically by [`PostId`]
	pub posts: Vec<PostView<'a>>,
}

impl std::ops::Deref for ThreadRef<'_> {
	type Target = Thread;
	fn deref(&self) -> &Self::Target { self.thread }
}


impl<'a> ThreadRef<'a> {
	pub fn id(&self) -> ThreadId { self.thread.id() }

	pub fn user(&self, user_entity: Entity) -> Result<&UserView<'a>> {
		self.users
			.iter()
			.find(|uv| uv.entity == user_entity)
			.ok_or_else(|| {
				bevyhow!(
					"No user for entity {user_entity} found in thread {thread:?}",
					thread = self.thread
				)
			})
	}
	pub fn user_from_id(&self, user_id: UserId) -> Result<&UserView<'a>> {
		self.users
			.iter()
			.find(|uv| uv.user.id() == user_id)
			.ok_or_else(|| {
				bevyhow!(
					"No user with id {user_id} found in thread {thread:?}",
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

	/// Find a stored [`ResponseMeta`] for the given user, provider, and model.
	pub fn stored_response(
		&self,
		user: Entity,
		provider_slug: &str,
		model_slug: &str,
	) -> Option<(&PostView<'_>, &ResponseMeta)> {
		self.posts.iter().find_map(|pv| {
			if pv.user_entity != user {
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
pub struct UserView<'a> {
	pub entity: Entity,
	pub user: &'a User,
}

impl std::ops::Deref for UserView<'_> {
	type Target = User;
	fn deref(&self) -> &Self::Target { self.user }
}

#[derive(Debug, Clone)]
pub struct PostView<'a> {
	pub entity: Entity,
	pub user_entity: Entity,
	pub post: &'a Post,
	pub user: &'a User,
	pub response_meta: Option<&'a ResponseMeta>,
}


impl std::ops::Deref for PostView<'_> {
	type Target = Post;
	fn deref(&self) -> &Self::Target { self.post }
}

impl PostView<'_> {
	pub fn entity(&self) -> Entity { self.entity }
	pub fn user_id(&self) -> UserId { self.user.id() }
}
