use crate::prelude::*;
use beet_core::prelude::*;
use beet_tool::prelude::*;
use bevy::app::Plugins;

#[derive(Debug)]
pub enum AsWorldMut<'a> {
	Owned(World),
	Borrowed(&'a mut World),
	OwnedApp(App),
	BorrowedApp(&'a mut App),
}
impl std::ops::Deref for AsWorldMut<'_> {
	type Target = World;
	fn deref(&self) -> &Self::Target {
		use AsWorldMut::*;
		match self {
			Owned(w) => w,
			Borrowed(w) => *w,
			OwnedApp(app) => app.world(),
			BorrowedApp(app) => app.world(),
		}
	}
}
impl std::ops::DerefMut for AsWorldMut<'_> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		use AsWorldMut::*;
		match self {
			Owned(w) => w,
			Borrowed(w) => *w,
			OwnedApp(app) => app.world_mut(),
			BorrowedApp(app) => app.world_mut(),
		}
	}
}

impl<'a> AsWorldMut<'a> {
	pub fn world_mut(&mut self) -> &mut World {
		use AsWorldMut::*;
		match self {
			Owned(w) => w,
			Borrowed(w) => *w,
			OwnedApp(app) => app.world_mut(),
			BorrowedApp(app) => app.world_mut(),
		}
	}
}

impl From<World> for AsWorldMut<'_> {
	fn from(world: World) -> Self { Self::Owned(world) }
}
impl<'a> From<&'a mut World> for AsWorldMut<'a> {
	fn from(world: &'a mut World) -> Self { Self::Borrowed(world) }
}
impl From<App> for AsWorldMut<'_> {
	fn from(app: App) -> Self { Self::OwnedApp(app) }
}
impl<'a> From<&'a mut App> for AsWorldMut<'a> {
	fn from(app: &'a mut App) -> Self { Self::BorrowedApp(app) }
}

#[derive(Debug)]
pub struct ThreadMut<'w> {
	world: AsWorldMut<'w>,
	id: ThreadId,
	entity: Entity,
}

impl Default for ThreadMut<'static> {
	fn default() -> Self { Self::new() }
}

impl ThreadMut<'static> {
	pub fn new() -> Self { Self::new_with_plugins(()) }
	pub fn new_logging<M>(level: Level) -> Self {
		Self::new_with_plugins(LogPlugin {
			level,
			filter: format!("ureq=off,ureq_proto=off"),
			..default()
		})
	}
	pub fn new_with_plugins<M>(plugins: impl Plugins<M>) -> Self {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, plugins))
			.init_plugin::<SocialPlugin>();

		Self::new_with_world(app)
	}
}

impl<'w> ThreadMut<'w> {
	pub fn new_with_world(world: impl Into<AsWorldMut<'w>>) -> Self {
		let mut world = world.into();
		let thread = Thread::new("Oneshot Thread");
		let id = thread.id();
		let entity = world.spawn(thread).id();

		Self { world, id, entity }
	}
	pub fn id(&self) -> ThreadId { self.id }
	pub fn entity(&self) -> Entity { self.entity }
	pub fn world(&self) -> &World { &self.world }
	pub fn world_mut(&mut self) -> &mut World { self.world.world_mut() }

	pub fn thread(&self) -> &Thread {
		self.world
			.entity(self.entity)
			.get::<Thread>()
			.expect("thread entity should have Thread component")
	}

	/// Inserts a [`User`] as a child of this thread and returns a mutable view.
	pub fn insert_user<'t>(&'t mut self, user: User) -> UserViewMut<'t, 'w> {
		let id = user.id();
		let entity = self.world.spawn((ChildOf(self.entity), user)).id();
		UserViewMut {
			thread_view: self,
			id,
			entity,
		}
	}

	pub fn user<'t>(&'t mut self, entity: Entity) -> UserViewMut<'t, 'w> {
		self.try_user(entity).unwrap()
	}

	pub fn try_user<'t>(
		&'t mut self,
		entity: Entity,
	) -> Result<UserViewMut<'t, 'w>> {
		let user = self
			.world
			.entity(entity)
			.get::<User>()
			.ok_or_else(|| {
				bevyhow!("Entity {entity:?} does not have a User component")
			})?
			.clone();
		Ok(UserViewMut {
			thread_view: self,
			id: user.id(),
			entity,
		})
	}

	pub fn user_from_id<'t>(
		&'t mut self,
		user_id: UserId,
	) -> UserViewMut<'t, 'w> {
		self.try_user_from_id(user_id).unwrap()
	}

	pub fn try_user_from_id<'t>(
		&'t mut self,
		user_id: UserId,
	) -> Result<UserViewMut<'t, 'w>> {
		let (entity, id) = self
			.world
			.with_state::<Query<(Entity, &User)>, _>(|query| {
				query
					.iter()
					.map(|(entity, user)| (entity, user.id()))
					.find(|(_, id)| id == &user_id)
			})
			.ok_or_else(|| {
				bevyhow!("User with id {user_id} not found in oneshot thread")
			})?;
		UserViewMut {
			id,
			entity,
			thread_view: self,
		}
		.xok()
	}
}

/// Mutable view into a [`User`] entity within a [`ThreadMut`].
///
/// `'t` is the lifetime of the borrow of [`ThreadMut`].
/// `'w` is the lifetime of the underlying world data.
pub struct UserViewMut<'t, 'w> {
	thread_view: &'t mut ThreadMut<'w>,
	id: UserId,
	entity: Entity,
}

impl<'t, 'w> UserViewMut<'t, 'w> {
	/// Inserts a [`Post`] as a child of this user and returns a mutable view.
	pub fn insert_post<'u>(
		&'u mut self,
		payload: impl Into<PostPayload>,
	) -> PostViewMut<'u, 't, 'w> {
		let post = Post::new(
			self.id,
			self.thread_view.id,
			PostStatus::Completed,
			payload,
		);
		let id = post.id();
		let entity = self
			.thread_view
			.world
			.spawn((ChildOf(self.entity), post))
			.id();
		PostViewMut {
			entity,
			id,
			user_view: self,
		}
	}

	pub fn world(&self) -> &World { &self.thread_view.world }
	pub fn world_mut(&mut self) -> &mut World {
		self.thread_view.world.world_mut()
	}
	pub fn id(&self) -> UserId { self.id }
	pub fn entity(&self) -> Entity { self.entity }

	/// Consumes this view and returns the underlying [`ThreadMut`] reference.
	pub fn thread_view(self) -> &'t mut ThreadMut<'w> { self.thread_view }

	pub fn user(&self) -> &User {
		self.thread_view
			.world
			.entity(self.entity)
			.get::<User>()
			.expect("user entity should have User component")
	}

	pub fn with_streamer(
		&mut self,
		streamer: impl Clone + Component + PostStreamer,
	) -> &mut Self {
		self.thread_view
			.world
			.entity_mut(self.entity)
			.insert(post_tool(streamer));
		self
	}

	pub async fn send_and_collect(&mut self) -> Result<Vec<Post>> {
		let post_ids = self.send().await?;
		self.thread_view
			.world
			.with_state::<Query<&Post>, _>(|query| {
				post_ids
					.iter()
					.filter_map(|post_id| {
						query.iter().find(|post| post.id() == *post_id).cloned()
					})
					.collect::<Vec<_>>()
			})
			.xok()
	}

	pub async fn send(&mut self) -> Result<Vec<PostId>> {
		let thread_id = self.thread_view.id;
		let existing_posts =
			self.world_mut().with_state::<Query<&Post>, _>(|query| {
				query
					.iter()
					.filter(|post| post.thread() == thread_id)
					.map(|post| post.id())
					.collect::<HashSet<_>>()
			});

		let _outcome = self
			.thread_view
			.world
			.entity_mut(self.entity)
			.call::<(), Outcome>(())
			.await?;

		let new_posts =
			self.world_mut().with_state::<Query<&Post>, _>(|posts| {
				posts
					.iter()
					.filter_map(|post| {
						if post.thread() == thread_id
							&& !existing_posts.contains(&post.id())
						{
							Some(post.id())
						} else {
							None
						}
					})
					.collect::<Vec<_>>()
			});

		Ok(new_posts)
	}
}

/// Mutable view into a [`Post`] entity within a [`UserViewMut`].
///
/// `'u` is the lifetime of the borrow of [`UserViewMut`].
/// `'t` is the lifetime of the borrow of [`ThreadMut`].
/// `'w` is the lifetime of the underlying world data.
pub struct PostViewMut<'u, 't, 'w> {
	user_view: &'u mut UserViewMut<'t, 'w>,
	id: PostId,
	entity: Entity,
}

impl<'u, 't, 'w> PostViewMut<'u, 't, 'w> {
	pub fn world(&self) -> &World { &self.user_view.thread_view.world }
	pub fn world_mut(&mut self) -> &mut World {
		self.user_view.thread_view.world.world_mut()
	}
	pub fn id(&self) -> PostId { self.id }
	pub fn entity(&self) -> Entity { self.entity }
	pub fn user_id(&self) -> UserId { self.user_view.id }
	pub fn thread_id(&self) -> ThreadId { self.user_view.thread_view.id }

	/// Consumes this view and returns the underlying [`ThreadMut`] reference.
	pub fn thread_view(self) -> &'t mut ThreadMut<'w>
	where
		'u: 't,
	{
		self.user_view.thread_view
	}

	/// Consumes this view and returns the underlying [`UserViewMut`] reference.
	pub fn user_view(self) -> &'u mut UserViewMut<'t, 'w> { self.user_view }

	pub fn post(&self) -> &Post {
		self.user_view
			.thread_view
			.world
			.entity(self.entity)
			.get::<Post>()
			.expect("post entity should have Post component")
	}
}


#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn compiles() {
		let mut world = World::new();
		world.spawn_empty();
		world.spawn_empty();

		let mut thread = ThreadMut::new();
		thread.insert_user(User::system());
		thread.insert_user(User::agent());
	}
}
