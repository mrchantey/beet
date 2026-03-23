use crate::prelude::*;
use beet_core::prelude::*;

#[derive(SystemParam)]
pub struct ThreadQuery<'w, 's> {
	pub commands: Commands<'w, 's>,
	pub ancestors: Query<'w, 's, &'static ChildOf>,
	pub children: Query<'w, 's, &'static Children>,
	pub threads: Query<'w, 's, (Entity, &'static Thread)>,
	pub users: Query<'w, 's, (Entity, &'static User)>,
	pub posts:
		Query<'w, 's, (Entity, &'static Post, Option<&'static ResponseMeta>)>,
}

impl<'w, 's> ThreadQuery<'w, 's> {
	/// Recurse up ancestors to find the [`Thread`] entity,
	/// then create a corresponding [`ThreadView`].
	/// Valid positions are:
	/// - any descendant of a thread, ie a User
	/// - any `PostOf`
	pub fn view(&self, entity: Entity) -> Result<ThreadView<'_>> {
		let (thread_entity, thread) = self
			.ancestors
			.iter_ancestors_inclusive(entity)
			.find_map(|ancestor| self.threads.get(ancestor).ok())
			.ok_or_else(|| bevyhow!("No Thread in ancestors of {entity:?}"))?;

		let users: Vec<UserView<'_>> = self
			.children
			.iter_descendants_inclusive(thread_entity)
			.filter_map(|entity| self.users.get(entity).ok())
			.map(|(entity, user)| UserView { entity, user })
			.collect();

		let mut posts: Vec<PostView<'_>> = self
			.children
			.iter_descendants_inclusive(thread_entity)
			.filter_map(|entity| self.posts.get(entity).ok())
			.xtry_map(|(entity, post, response_meta)| -> Result<PostView> {
				let user = self.user_from_post_entity(entity)?;
				PostView {
					entity,
					post,
					user: user.user,
					user_entity: user.entity,
					response_meta,
				}
				.xok()
			})?;
		posts.sort_by_key(|pv| pv.post.id());

		ThreadView {
			entity: thread_entity,
			thread,
			users,
			posts,
		}
		.xok()
	}

	/// Find the [`UserView`] that owns the given post entity.
	pub fn user_from_post_entity<'a>(
		&'a self,
		post: Entity,
	) -> Result<UserView<'a>> {
		self.ancestors
			.iter_ancestors_inclusive(post)
			.find_map(|entity| self.users.get(entity).ok())
			.map(|(entity, user)| UserView { entity, user })
			.ok_or_else(|| bevyhow!("No user ancestor found for post {post:?}"))
	}

	pub fn spawn_post(
		&mut self,
		parent: Entity,
		status: PostStatus,
		payload: impl Into<PostPayload>,
	) -> Result<Entity> {
		let user_id = self
			.ancestors
			.iter_ancestors_inclusive(parent)
			.find_map(|entity| {
				self.users.get(entity).map(|(_, user)| user.id()).ok()
			})
			.ok_or_else(|| bevyhow!("No user ancestor found for {parent:?}"))?;
		let thread_id = self
			.ancestors
			.iter_ancestors_inclusive(parent)
			.find_map(|entity| {
				self.threads.get(entity).map(|(_, thread)| thread.id()).ok()
			})
			.ok_or_else(|| {
				bevyhow!("No thread ancestor found for {parent:?}")
			})?;
		self.commands
			.spawn((
				ChildOf(parent),
				Post::new(user_id, thread_id, status, payload),
			))
			.id()
			.xok()
	}
}
#[derive(Debug, Clone)]
pub struct ThreadView<'a> {
	pub entity: Entity,
	pub thread: &'a Thread,
	/// The list of users in bfs order of [`Children`]
	pub users: Vec<UserView<'a>>,
	/// The list of posts in this thread, sorted chronologically by [`PostId`]
	pub posts: Vec<PostView<'a>>,
}

impl std::ops::Deref for ThreadView<'_> {
	type Target = Thread;
	fn deref(&self) -> &Self::Target { self.thread }
}


impl<'a> ThreadView<'a> {
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
