use crate::prelude::*;
use beet_core::prelude::*;

#[derive(SystemParam)]
pub struct SocialQuery<'w, 's> {
	pub commands: Commands<'w, 's>,
	pub ancestors: Query<'w, 's, &'static ChildOf>,
	pub children: Query<'w, 's, &'static Children>,
	pub threads: Query<'w, 's, (Entity, &'static Thread)>,
	pub users: Query<'w, 's, (Entity, &'static User)>,
	pub posts:
		Query<'w, 's, (Entity, &'static Post, Option<&'static ResponseMeta>)>,
}

impl<'w, 's> SocialQuery<'w, 's> {
	/// Recurse up ancestors to find the [`Thread`] entity,
	/// then create a corresponding [`ThreadRef`].
	/// Valid positions are:
	/// - any descendant of a thread, ie a User
	/// - any `PostOf`
	pub fn thread(&self, entity: Entity) -> Result<ThreadRef<'_>> {
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

		ThreadRef {
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
