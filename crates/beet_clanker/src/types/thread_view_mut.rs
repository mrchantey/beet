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
pub struct ThreadViewMut<'a> {
	world: AsWorldMut<'a>,
	entity: Entity,
	actors: Vec<Actor>,
	actions: Vec<Action>,
}


impl ThreadViewMut<'static> {
	pub fn new<M>() -> Self { Self::new_with_plugins(()) }
	pub fn new_logging<M>(level: Level) -> Self {
		Self::new_with_plugins(LogPlugin {
			level,
			filter: format!("{}=trace,ureq=off,ureq_proto=off", module_path!()),
			..default()
		})
	}
	pub fn new_with_plugins<M>(plugins: impl Plugins<M>) -> Self {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, plugins))
			.init_plugin::<ClankerPlugin>();

		Self::new_with_world(app)
	}
}

impl<'a> ThreadViewMut<'a> {
	pub fn new_with_world(world: impl Into<AsWorldMut<'a>>) -> Self {
		let mut world = world.into();
		let thread_entity = world.spawn(Thread::new("Oneshot Thread")).id();

		Self {
			world,
			entity: thread_entity,
			actors: Vec::new(),
			actions: Vec::new(),
		}
	}
	pub fn thread(&self) -> Thread {
		self.world
			.entity(self.entity)
			.get::<Thread>()
			.expect("thread entity should have Thread component")
			.clone()
	}

	pub fn insert_actor(&'a mut self, actor: Actor) -> ActorViewMut<'a> {
		let id = actor.id();
		let entity = self.world.spawn((ChildOf(self.entity), actor)).id();
		ActorViewMut {
			thread_view: self,
			id,
			entity,
		}
	}
	pub fn actor(&'a mut self, actor_id: ActorId) -> ActorViewMut<'a> {
		self.try_actor(actor_id).unwrap()
	}
	pub fn try_actor(
		&'a mut self,
		actor_id: ActorId,
	) -> Result<ActorViewMut<'a>> {
		let (entity, id) = self
			.world
			.with_state::<Query<(Entity, &Actor)>, _>(|query| {
				query
					.iter()
					.map(|(entity, actor)| (entity, actor.id()))
					.find(|(_, id)| id == &actor_id)
			})
			.ok_or_else(|| {
				bevyhow!("Actor with id {actor_id} not found in oneshot thread")
			})?;
		ActorViewMut {
			id,
			entity,
			thread_view: self,
		}
		.xok()
	}
}


pub struct ActorViewMut<'a> {
	thread_view: &'a mut ThreadViewMut<'a>,
	entity: Entity,
	id: ActorId,
}

impl<'a> ActorViewMut<'a> {
	pub fn insert_action(
		&mut self,
		payload: impl Into<ActionPayload>,
	) -> &mut Self {
		self.thread_view.actions.push(Action::new(
			self.actor_id,
			self.thread_view.thread.id(),
			ActionStatus::Completed,
			payload,
		));
		self
	}
	pub fn id(&self) -> ActorId { self.actor_id }
	pub fn thread(self) -> &'a mut ThreadViewMut { self.thread_view }
	pub fn actor(&self) -> &Actor {
		self.thread_view
			.actors
			.iter()
			.find(|a| a.id() == self.actor_id)
			.expect("creating a OneshotActor is already guarded")
	}


	pub async fn send(
		&mut self,
		streamer: impl Clone + Component + ActionStreamer,
	) -> Result<Vec<ActionId>> {
		let existing_actions = self
			.thread_view
			.actions
			.iter()
			.map(|action| action.id())
			.collect::<HashSet<_>>();

		let world = app.world_mut();
		let thread_id = self.thread_view.thread.id();
		let thread_entity = world.spawn(self.thread_view.thread.clone()).id();

		let _ = world
			.spawn((
				ChildOf(thread_entity),
				self.actor().clone(),
				action_tool(streamer),
			))
			.id();
		world.entity().call::<(), Outcome>(()).await?;

		let new_actions =
			world.with_state::<Query<&Action>, _>(move |actions| {
				actions
					.iter()
					.filter_map(|action| {
						if action.thread() == thread_id
							&& !existing_actions.contains(&action.id())
						{
							Some(action.id())
						} else {
							None
						}
					})
					.collect::<Vec<_>>()
			});


		Ok(new_actions)
	}
}
