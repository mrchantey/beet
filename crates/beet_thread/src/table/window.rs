//! Windows onto one stored thread: the feed carrying each window's posts to
//! the others, the canonical window holding its behavior, and the actors
//! each window holds.
use crate::prelude::*;
#[cfg(feature = "action")]
use beet_action::prelude::*;
use beet_core::prelude::*;
#[cfg(feature = "action")]
use beet_net::prelude::*;

/// The actors a window's own scene authored, which it holds: a window joins
/// only an actor no other window of the runtime holds.
#[derive(Debug, Default, Clone, Component)]
pub struct PresentActors(Vec<ActorId>);

impl PresentActors {
	pub fn new(actors: Vec<ActorId>) -> Self { Self(actors) }
	pub fn contains(&self, actor: ActorId) -> bool { self.0.contains(&actor) }
}

/// The windows of the world, found by the id of the stored thread they are
/// open onto, never by walking from one window to another.
#[derive(SystemParam)]
pub struct ThreadWindows<'w, 's> {
	windows: Query<
		'w,
		's,
		(Entity, &'static Thread, Option<&'static PresentActors>),
	>,
	#[cfg(feature = "action")]
	children: Query<'w, 's, &'static Children>,
	#[cfg(feature = "action")]
	behaviors: Query<
		'w,
		's,
		(),
		(With<ActionMeta>, Or<(With<Actor>, With<ActorRef>)>),
	>,
}

impl ThreadWindows<'_, '_> {
	/// Every window open onto the stored thread `id` but `except`.
	pub fn of(
		&self,
		id: ThreadId,
		except: Entity,
	) -> impl Iterator<Item = Entity> {
		self.windows
			.iter()
			.filter(move |(entity, thread, _)| {
				*entity != except && thread.id() == id
			})
			.map(|(entity, ..)| entity)
	}

	/// Whether the window `entity` holds behavior: an action on an actor in
	/// its scene.
	#[cfg(feature = "action")]
	pub fn holds_behavior(&self, entity: Entity) -> bool {
		self.children
			.iter_descendants(entity)
			.any(|child| self.behaviors.contains(child))
	}

	/// The canonical window of the stored thread `id`, the one holding its
	/// behavior, other than `except`.
	#[cfg(feature = "action")]
	pub fn canonical(&self, id: ThreadId, except: Entity) -> Option<Entity> {
		self.of(id, except)
			.find(|window| self.holds_behavior(*window))
	}

	/// Whether a window of the stored thread `id` other than `except` holds
	/// `actor`.
	pub fn holds_actor(
		&self,
		id: ThreadId,
		except: Entity,
		actor: ActorId,
	) -> bool {
		self.of(id, except).any(|window| {
			self.windows
				.get(window)
				.ok()
				.and_then(|(_, _, present)| present)
				.is_some_and(|present| present.contains(actor))
		})
	}

	/// Refuse the window `entity` opening onto `id` when it holds behavior
	/// and another window already does, naming both: two agents answering is
	/// a mistake in what was installed, so a [`ThreadRoute`] answers it `409`.
	#[cfg(feature = "action")]
	pub fn assert_canonical(&self, id: ThreadId, entity: Entity) -> Result {
		if !self.holds_behavior(entity) {
			return Ok(());
		}
		match self.canonical(id, entity) {
			Some(canonical) => Err(HttpError::new(
				StatusCode::CONFLICT,
				format!(
					"the thread {id} already runs its behavior in the window \
					 {canonical}, so the window {entity} holding behavior for \
					 it is refused"
				),
			)
			.into()),
			None => Ok(()),
		}
	}
}

/// The feed: each window's changed posts, in progress or completed, applied
/// to every other window of the same stored thread, each received post marked
/// written so only its author's window ever flushes it.
pub(crate) fn feed_windows(
	mut windows: Query<(
		Entity,
		&Thread,
		&mut ThreadRecords,
		Option<&mut SyncedPosts>,
	)>,
) {
	let mut sent = Vec::new();
	for (entity, thread, mut records, _) in windows.iter_mut() {
		if records.has_feed() {
			let feed = records.bypass_change_detection().take_feed();
			sent.push((entity, thread.id(), feed));
		}
	}
	for (source, id, feed) in sent {
		for (_, _, mut records, synced) in
			windows.iter_mut().filter(|(entity, thread, ..)| {
				*entity != source && thread.id() == id
			}) {
			records.receive(&feed);
			if let Some(mut synced) = synced {
				for post in &feed.posts {
					synced.insert(post.id());
				}
			}
		}
	}
}

#[cfg(all(test, feature = "action"))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// An app over a store, `windows` spawned beneath it.
	fn app() -> (App, Entity) {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.install_type::<ThreadPlugin>();
		let root = app.world_mut().spawn(InMemoryStore::new()).id();
		(app, root)
	}

	/// A window over the store at `root` mounting `threads`: a person who
	/// posts nothing, and `agent` if given.
	fn window(app: &mut App, root: Entity, agent: bool) -> Entity {
		let mut window = app.world_mut().spawn((
			ChildOf(root),
			Thread::default(),
			MountThreadStore {
				path: RelPath::new("threads"),
				new: false,
			},
			children![Actor::new_with_id(
				ActorId::from_u128(1),
				"Person",
				ActorKind::User,
			)],
		));
		if agent {
			window.with_child((
				Actor::new_with_id(
					ActorId::from_u128(2),
					"Agent",
					ActorKind::Agent,
				),
				MockPostStreamer::default(),
			));
		}
		window.id()
	}

	/// Enter the window `entity`, answering the entry's outcome.
	async fn enter(app: &mut App, entity: Entity) -> Result {
		app.world_mut()
			.run_async_local_then(async move |world: AsyncWorld| -> Result {
				Thread::enter(
					world.clone(),
					entity,
					ThreadEntry::parse(Request::get("chat").parts().clone())?,
				)
				.await
			})
			.await
	}

	/// Push a completed text post by the person into `window`'s records.
	fn post(app: &mut App, window: Entity, text: &str) {
		let mut entity = app.world_mut().entity_mut(window);
		let id = entity.get::<Thread>().unwrap().id();
		entity.get_mut::<ThreadRecords>().unwrap().upsert_post(
			AgentPost::new_text(
				ActorId::from_u128(1),
				id,
				text,
				PostStatus::Completed,
			),
		);
	}

	/// The text posts of `window`, its joins aside.
	fn texts(app: &App, window: Entity) -> Vec<String> {
		app.world()
			.get::<ThreadRecords>(window)
			.unwrap()
			.model_posts(None)
			.iter()
			.map(|view| view.post.to_string())
			.collect()
	}

	fn pump(app: &mut App) {
		for _ in 0..10 {
			app.update();
		}
	}

	/// Two windows on one stored thread each post, both see every post in
	/// order, and the store holds each once.
	#[beet_core::test]
	async fn two_windows_see_each_others_posts() {
		let (mut app, root) = app();
		let first = window(&mut app, root, false);
		let second = window(&mut app, root, false);
		enter(&mut app, first).await.unwrap();
		enter(&mut app, second).await.unwrap();
		let id = app.world().get::<Thread>(first).unwrap().id();
		app.world().get::<Thread>(second).unwrap().id().xpect_eq(id);
		post(&mut app, first, "one");
		pump(&mut app);
		post(&mut app, second, "two");
		pump(&mut app);
		post(&mut app, first, "three");
		pump(&mut app);
		let expected = ["one", "two", "three"].map(String::from).to_vec();
		texts(&app, first).xpect_eq(expected.clone());
		texts(&app, second).xpect_eq(expected.clone());

		let store = app.world().get::<ThreadStore>(first).unwrap().clone();
		for window in [first, second] {
			app.world_mut()
				.run_async_local_then(async move |world: AsyncWorld| {
					ThreadStore::flush(&world.entity(window)).await
				})
				.await
				.unwrap();
		}
		store
			.posts(id)
			.await
			.unwrap()
			.into_iter()
			.filter(|post| post.intent() != PostIntent::Log)
			.map(|post| post.to_string())
			.collect::<Vec<_>>()
			.xpect_eq(expected);
	}

	/// A post in progress streams into a second window before it completes,
	/// and neither writes it until it does.
	#[beet_core::test]
	async fn a_post_in_progress_streams_into_every_window() {
		let (mut app, root) = app();
		let first = window(&mut app, root, false);
		let second = window(&mut app, root, false);
		enter(&mut app, first).await.unwrap();
		enter(&mut app, second).await.unwrap();
		let id = app.world().get::<Thread>(first).unwrap().id();
		let mut streaming = AgentPost::new_text(
			ActorId::from_u128(1),
			id,
			"hel",
			PostStatus::InProgress,
		);
		app.world_mut()
			.get_mut::<ThreadRecords>(first)
			.unwrap()
			.upsert_post(streaming.clone());
		pump(&mut app);
		texts(&app, second).xpect_eq(vec!["hel".to_string()]);
		app.world()
			.get::<ThreadRecords>(second)
			.unwrap()
			.last_post()
			.unwrap()
			.in_progress()
			.xpect_true();
		let store = app.world().get::<ThreadStore>(first).unwrap().clone();
		store
			.posts(id)
			.await
			.unwrap()
			.iter()
			.any(|post| post.id() == streaming.id())
			.xpect_false();

		streaming.set_text("hello");
		streaming.set_status(PostStatus::Completed);
		app.world_mut()
			.get_mut::<ThreadRecords>(first)
			.unwrap()
			.upsert_post(streaming);
		pump(&mut app);
		texts(&app, second).xpect_eq(vec!["hello".to_string()]);
		app.world()
			.get::<ThreadRecords>(second)
			.unwrap()
			.last_post()
			.unwrap()
			.in_progress()
			.xpect_false();
	}

	/// A second window carrying behavior for a thread with a canonical window
	/// is refused, naming both, while a window without behavior joins.
	#[beet_core::test]
	async fn a_second_canonical_window_is_refused() {
		let (mut app, root) = app();
		let canonical = window(&mut app, root, true);
		let view = window(&mut app, root, false);
		let rival = window(&mut app, root, true);
		enter(&mut app, canonical).await.unwrap();
		enter(&mut app, view).await.unwrap();
		enter(&mut app, rival)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains(format!(
				"already runs its behavior in the window {canonical}, so the window {rival}"
			));
	}

	/// The same person in two windows joins once, and an agent only the
	/// second window holds joins there.
	#[beet_core::test]
	async fn the_same_person_joins_once() {
		let (mut app, root) = app();
		let first = window(&mut app, root, false);
		let second = window(&mut app, root, true);
		enter(&mut app, first).await.unwrap();
		enter(&mut app, second).await.unwrap();
		pump(&mut app);
		for window in [first, second] {
			app.world()
				.get::<ThreadRecords>(window)
				.unwrap()
				.posts()
				.iter()
				.filter(|post| SessionOpened::opens(post))
				.map(Post::author)
				.collect::<Vec<_>>()
				.xpect_eq(vec![ActorId::from_u128(1), ActorId::from_u128(2)]);
		}
	}
}
