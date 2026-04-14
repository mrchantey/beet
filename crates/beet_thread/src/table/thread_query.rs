use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

#[derive(SystemParam)]
pub struct ThreadQuery<'w, 's> {
	pub commands: Commands<'w, 's>,
	pub ancestors: Query<'w, 's, &'static ChildOf>,
	pub children: Query<'w, 's, &'static Children>,
	pub threads:
		Query<'w, 's, (Entity, &'static Thread, Option<&'static Blob>)>,
	pub actors:
		Query<'w, 's, (Entity, &'static Actor, Option<&'static ToolChoice>)>,
	pub tools: Query<'w, 's, (Entity, &'static ToolDefinition)>,
	pub posts:
		Query<'w, 's, (Entity, &'static Post, Option<&'static ResponseMeta>)>,
}

impl<'w, 's> ThreadQuery<'w, 's> {
	/// Recurse up ancestors to find the [`Thread`] entity,
	/// then create a corresponding [`ThreadRef`].
	/// Valid positions are:
	/// - any descendant of a thread, ie an Actor or Post
	/// - any `PostOf`
	pub fn thread(&self, entity: Entity) -> Result<ThreadRef<'_>> {
		let (thread_entity, thread, blob) = self
			.ancestors
			.iter_ancestors_inclusive(entity)
			.find_map(|ancestor| self.threads.get(ancestor).ok())
			.ok_or_else(|| bevyhow!("No Thread in ancestors of {entity:?}"))?;

		let actors: Vec<ActorView<'_>> = self
			.children
			.iter_descendants_inclusive(thread_entity)
			.filter_map(|entity| self.actors.get(entity).ok())
			.map(|(entity, actor, tool_choice)| ActorView {
				entity,
				actor,
				tool_choice,
			})
			.collect();

		let mut posts: Vec<PostView<'_>> = self
			.children
			.iter_descendants_inclusive(thread_entity)
			.filter_map(|entity| self.posts.get(entity).ok())
			.xtry_map(|(entity, post, response_meta)| -> Result<PostView> {
				let actor = self.actor_from_post_entity(entity)?;
				PostView {
					entity,
					post,
					actor: actor.actor,
					actor_entity: actor.entity,
					response_meta,
				}
				.xok()
			})?;
		posts.sort_by_key(|pv| pv.post.id());

		ThreadRef {
			entity: thread_entity,
			thread,
			blob,
			actors,
			posts,
		}
		.xok()
	}
	/// Recurse down (DFS) to find all [`ToolDefinition`] entities, returning their entities and definitions.
	pub fn tools(&self, actor: Entity) -> Vec<(Entity, &ToolDefinition)> {
		self.children
			.iter_descendants_inclusive(actor)
			.filter_map(|entity| self.tools.get(entity).ok())
			.collect()
	}

	/// Find the [`ActorView`] that owns the given post entity.
	pub fn actor_from_post_entity<'a>(
		&'a self,
		post: Entity,
	) -> Result<ActorView<'a>> {
		self.ancestors
			.iter_ancestors_inclusive(post)
			.find_map(|entity| self.actors.get(entity).ok())
			.map(|(entity, actor, tool_choice)| ActorView {
				entity,
				actor,
				tool_choice,
			})
			.ok_or_else(|| {
				bevyhow!("No actor ancestor found for post {post:?}")
			})
	}

	/// Spawn a new text post under the given parent entity,
	/// resolving the actor and thread from ancestors.
	pub fn spawn_post(
		&mut self,
		parent: Entity,
		status: PostStatus,
		content: impl Into<IntoPost>,
	) -> Result<Entity> {
		let actor_id = self
			.ancestors
			.iter_ancestors_inclusive(parent)
			.find_map(|entity| {
				self.actors.get(entity).map(|(_, actor, _)| actor.id()).ok()
			})
			.ok_or_else(|| {
				bevyhow!("No actor ancestor found for {parent:?}")
			})?;
		let thread_id = self
			.ancestors
			.iter_ancestors_inclusive(parent)
			.find_map(|entity| {
				self.threads
					.get(entity)
					.map(|(_, thread, _)| thread.id())
					.ok()
			})
			.ok_or_else(|| {
				bevyhow!("No thread ancestor found for {parent:?}")
			})?;
		let mut post = content.into().into_post(actor_id, thread_id);
		post.set_status(status);
		self.commands.spawn((ChildOf(parent), post)).id().xok()
	}
}
