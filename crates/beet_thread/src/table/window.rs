//! Windows onto one stored thread: window sync applying each window's changes
//! to the others, the canonical window holding its behavior, and the actors
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
	pub fn contains(&self, actor: &ActorId) -> bool { self.0.contains(actor) }
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
	#[cfg(feature = "action")]
	actor_refs: Query<'w, 's, &'static ActorRef, With<ActionMeta>>,
	#[cfg(feature = "action")]
	compacts: Query<'w, 's, &'static Compact>,
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

	/// Whether the window `entity` holds an agent's behavior, an action on an
	/// actor of kind Agent in its scene, whose context `records` resolve.
	#[cfg(feature = "action")]
	pub fn holds_agent(&self, entity: Entity, records: &ThreadRecords) -> bool {
		self.children
			.iter_descendants(entity)
			.filter_map(|child| self.actor_refs.get(child).ok())
			.any(|actor| {
				records
					.actor(actor)
					.is_ok_and(|actor| actor.kind() == ActorKind::Agent)
			})
	}

	/// The agents of the window `entity` when every one compacts by
	/// [`CompactMethod::Rolling`], so the window reads back through their
	/// compactions ([`Reach::Compacted`]); `None` when it holds none, or one
	/// that reads the whole thread.
	#[cfg(feature = "action")]
	pub fn compacting_agents(
		&self,
		entity: Entity,
		records: &ThreadRecords,
	) -> Option<Vec<ActorId>> {
		let mut agents = Vec::new();
		for child in self.children.iter_descendants(entity) {
			let Ok(actor) = self.actor_refs.get(child) else {
				continue;
			};
			if !records
				.actor(actor)
				.is_ok_and(|actor| actor.kind() == ActorKind::Agent)
			{
				continue;
			}
			match self.compacts.get(child) {
				Ok(compact) if compact.method == CompactMethod::Rolling => {
					agents.push(actor.0.clone());
				}
				_ => return None,
			}
		}
		(!agents.is_empty()).then_some(agents)
	}

	/// The behavior entity of every actor in every window of the stored
	/// thread `id`.
	#[cfg(feature = "action")]
	pub fn actors(&self, id: ThreadId) -> impl Iterator<Item = Entity> {
		self.windows
			.iter()
			.filter(move |(_, thread, _)| thread.id() == id)
			.flat_map(|(window, ..)| self.children.iter_descendants(window))
			.filter(|child| self.behaviors.contains(*child))
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
		actor: &ActorId,
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

/// Window sync: each window's changed posts, in progress or completed, applied
/// to every other window of the same stored thread, each received post marked
/// written so only its author's window ever flushes it.
pub(crate) fn sync_windows(
	mut windows: Query<(
		Entity,
		&Thread,
		&mut ThreadRecords,
		Option<&mut SyncedRecords>,
	)>,
) {
	let mut sent = Vec::new();
	for (entity, thread, mut records, _) in windows.iter_mut() {
		if records.has_changes() {
			let changes = records.bypass_change_detection().take_changes();
			sent.push((entity, thread.id(), changes));
		}
	}
	for (source, id, changes) in sent {
		for (_, _, mut records, synced) in
			windows.iter_mut().filter(|(entity, thread, ..)| {
				*entity != source && thread.id() == id
			}) {
			records.apply_changes(&changes);
			if let Some(mut synced) = synced {
				synced.receive(&changes);
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
			.install_types::<ThreadPlugin>();
		let root = app
			.world_mut()
			.spawn(TableStore::new(InMemoryTableStore::new()))
			.id();
		(app, root)
	}

	/// A window over the store at `root` mounting `threads`: a person who
	/// posts nothing, and `agent` if given.
	fn window(app: &mut App, root: Entity, agent: bool) -> Entity {
		let mut window = app.world_mut().spawn((
			ChildOf(root),
			Thread::default(),
			MountThreadStore::new(fwd_path!("threads")),
			children![Actor::new_with_id(
				actor_id!("actor-1"),
				"Person",
				ActorKind::User,
			)],
		));
		if agent {
			window.with_child((
				Actor::new_with_id(
					actor_id!("actor-2"),
					"Agent",
					ActorKind::Agent,
				),
				MockPostStreamer::default(),
			));
		}
		window.id()
	}

	/// A window as [`window`] makes it, reading `load` posts at a time and
	/// its person waiting on the records, the window's behavior, when
	/// `person_turn`.
	fn window_loading(
		app: &mut App,
		root: Entity,
		agent: bool,
		person_turn: bool,
		load: usize,
	) -> Entity {
		let entity = window(app, root, agent);
		let mut mount = MountThreadStore::new(fwd_path!("threads"));
		mount.load = load;
		app.world_mut().entity_mut(entity).insert(mount);
		if person_turn {
			let person = app.world().get::<Children>(entity).unwrap()[0];
			app.world_mut().entity_mut(person).insert(UserInput);
		}
		entity
	}

	/// The person of every window.
	fn person() -> Actor {
		Actor::new_with_id(
			actor_id!("actor-1"),
			"Person",
			ActorKind::User,
		)
	}

	/// Store a thread under `threads` at `root`: a seed of two posts, then
	/// `count` numbered posts, all by the person.
	async fn store_long_thread(
		app: &App,
		root: Entity,
		count: usize,
	) -> Thread {
		let table = app.world().get::<TableStore>(root).unwrap().clone();
		let store = ThreadStore::new(table, fwd_path!("threads")).unwrap();
		let mut thread = Thread::new("Long");
		let id = thread.id();
		let text = |text: String| {
			AgentPost::new_text(
				person().id().clone(),
				id,
				text,
				PostStatus::Completed,
			)
		};
		let mut posts = vec![text("seed 0".into()), text("seed 1".into())];
		thread.set_seed(Some(posts[1].id()));
		posts.extend((0..count).map(|i| text(format!("post {i}"))));
		store
			.store_thread(&thread, vec![person()], posts)
			.await
			.unwrap();
		thread
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
				actor_id!("actor-1"),
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
			.content_posts()
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
			.filter(|post| !post.is_log())
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
			actor_id!("actor-1"),
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

	/// A view's window opens on the newest posts, a canonical one holds the
	/// seed too, and one holding an agent opens on the whole thread, its
	/// agent's context.
	#[beet_core::test]
	async fn a_window_opens_as_far_back_as_it_reads() {
		async fn opened(agent: bool, person_turn: bool) -> Vec<String> {
			let (mut app, root) = app();
			store_long_thread(&app, root, 30).await;
			let entity = window_loading(&mut app, root, agent, person_turn, 3);
			enter(&mut app, entity).await.unwrap();
			let records = app.world().get::<ThreadRecords>(entity).unwrap();
			records.has_older().xpect_eq(!agent);
			texts(&app, entity)
		}
		opened(false, false).await.xpect_eq(
			["post 27", "post 28", "post 29"].map(String::from).to_vec(),
		);
		opened(false, true).await.xpect_eq(
			["seed 0", "seed 1", "post 27", "post 28", "post 29"]
				.map(String::from)
				.to_vec(),
		);
		opened(true, false).await.len().xpect_eq(32);
	}

	/// A change to a post a window has not loaded is dropped by it, while a
	/// new post reaches it.
	#[beet_core::test]
	async fn a_window_drops_a_change_it_has_not_loaded() {
		let (mut app, root) = app();
		store_long_thread(&app, root, 30).await;
		let whole = window_loading(&mut app, root, true, false, 3);
		let view = window_loading(&mut app, root, false, false, 3);
		enter(&mut app, whole).await.unwrap();
		enter(&mut app, view).await.unwrap();
		let mut changed = app
			.world()
			.get::<ThreadRecords>(whole)
			.unwrap()
			.content_posts()
			.next()
			.unwrap()
			.post
			.clone();
		changed.set_text("seed 0, changed");
		app.world_mut()
			.get_mut::<ThreadRecords>(whole)
			.unwrap()
			.upsert_post(changed);
		post(&mut app, whole, "new");
		pump(&mut app);
		// the newest three: the last post and the whole window's two joins
		texts(&app, view)
			.xpect_eq(["post 29", "new"].map(String::from).to_vec());
		texts(&app, whole)[0].xpect_eq("seed 0, changed");
	}

	/// A window joining a live one takes the posts the store may not hold
	/// yet, one still in progress among them, and reads the rest from the
	/// store, never the live window's older posts.
	#[beet_core::test]
	async fn a_joining_window_takes_unstored_posts() {
		let (mut app, root) = app();
		let thread = store_long_thread(&app, root, 30).await;
		let whole = window_loading(&mut app, root, true, false, 3);
		enter(&mut app, whole).await.unwrap();
		app.world_mut()
			.get_mut::<ThreadRecords>(whole)
			.unwrap()
			.upsert_post(AgentPost::new_text(
				person().id().clone(),
				thread.id(),
				"typing",
				PostStatus::InProgress,
			));
		let view = window_loading(&mut app, root, false, false, 3);
		enter(&mut app, view).await.unwrap();
		texts(&app, view)
			.xpect_eq(["post 29", "typing"].map(String::from).to_vec());
		let store = app.world().get::<ThreadStore>(whole).unwrap().clone();
		store
			.posts(thread.id())
			.await
			.unwrap()
			.iter()
			.any(|post| post.to_string() == "typing")
			.xpect_false();
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
				.map(|post| post.author().clone())
				.collect::<Vec<_>>()
				.xpect_eq(vec![
					actor_id!("actor-1"),
					actor_id!("actor-2"),
				]);
		}
	}
}
