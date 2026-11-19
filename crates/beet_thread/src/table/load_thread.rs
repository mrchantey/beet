//! Entering a thread: which stored thread a session continues, and the
//! window it continues with.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Declare a thread-record store from markup, persisting the thread's records
/// to a single JSON object under `path` in the store at or above the thread,
/// so a later session continues rather than starting fresh:
/// `<Thread {(Sequence, MountThreadStore{path:"threads/chat"})}>`.
///
/// This is a *deferred* mount: [`Thread::enter`] builds the [`ThreadStore`],
/// picks the stored thread the entry continues ([`ThreadEntry`]), and only
/// then inserts the store onto the thread, so the persistence sync never
/// flushes a fresh, un-adopted thread. One path keeps every thread entered
/// under it, each found by its metadata, ie the threads a [`ThreadRoute`]
/// builds per request. The store it scopes is the same one an agent's tools
/// resolve, so the records sit beside whatever else the scene keeps there,
/// under their own path.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct MountThreadStore {
	/// The directory the thread's records persist under, in the store at or
	/// above the thread.
	pub path: RelPath,
	/// How many of the newest posts a continued thread enters with, its
	/// window bound ([`ThreadStoreProvider::materialize_window`]), or its
	/// whole history when `None`.
	pub tail: Option<u64>,
}

impl Default for MountThreadStore {
	fn default() -> Self {
		Self {
			path: default(),
			tail: Some(Self::DEFAULT_TAIL),
		}
	}
}

impl MountThreadStore {
	/// The posts a continued thread enters with unless its mount says
	/// otherwise.
	pub const DEFAULT_TAIL: u64 = 200;

	/// Build the [`ThreadStore`] this declares: a single-JSON-object
	/// [`BlobThreadStore`] under `path` in `store`.
	pub fn build(&self, store: &BlobStore) -> ThreadStore {
		ThreadStore::new(BlobThreadStore::new(
			store.with_subdir(self.path.clone()),
		))
	}
}

/// The params a session's entry reads off its request, taken by
/// [`ThreadRoute`] and [`RunThread`], so `--help` lists them and a strict
/// server validates them.
#[derive(Debug, Default, Clone, Reflect)]
pub struct ThreadEntryParams {
	/// Start a fresh thread rather than continuing a stored one, stamped the
	/// same, so the next request continues it.
	pub new: bool,
	/// Continue this stored thread, whatever its metadata.
	pub thread: Option<ThreadId>,
	/// The post whose action asked for this session, `<thread id>/<post id>`:
	/// a thread the request creates records it as [`Thread::spawned_by`], and
	/// the session's opened post as its cause.
	pub spawned_by: Option<PostRef>,
}

/// How one session enters a thread: the stored thread it continues, what a
/// thread it creates is stamped with, and what caused it.
///
/// Continuation is a match on the thread's metadata the route declares,
/// never its seed: an entry declaring metadata continues the newest stored
/// thread whose metadata holds every entry of it, and one declaring none
/// creates a thread every time. [`ThreadEntryParams::thread`] continues that
/// thread explicitly, [`ThreadEntryParams::new`] creates one, stamped the
/// same.
#[derive(Debug, Default, Clone)]
pub struct ThreadEntry {
	/// The entries a created thread is stamped with.
	pub metadata: Map,
	/// Whether a stored thread holding every entry of `metadata` is
	/// continued, rather than a thread created every time.
	pub continues: bool,
	/// The params of the request entering.
	pub params: ThreadEntryParams,
}

impl ThreadEntry {
	/// An entry stamping `metadata`, continuing by it unless it is empty.
	pub fn new(metadata: Map, params: ThreadEntryParams) -> Self {
		Self {
			continues: !metadata.is_empty(),
			metadata,
			params,
		}
	}

	/// The stored thread this entry continues, if any.
	///
	/// ## Errors
	///
	/// Errors when [`ThreadEntryParams::thread`] names a thread the store
	/// lacks.
	pub async fn find(&self, store: &ThreadStore) -> Result<Option<Thread>> {
		if self.params.new {
			return Ok(None);
		}
		if let Some(id) = self.params.thread {
			return store
				.thread(id)
				.await?
				.ok_or_else(|| bevyhow!("the store holds no thread {id}"))?
				.xsome()
				.xok();
		}
		if !self.continues {
			return Ok(None);
		}
		store
			.threads()
			.await?
			.into_iter()
			.filter(|thread| thread.matches(&self.metadata))
			.max_by_key(|thread| (thread.created(), thread.id()))
			.xok()
	}

	/// Stamp a thread this entry creates.
	fn stamp(&self, thread: &mut Thread) {
		for (key, value) in self.metadata.iter() {
			thread.metadata_mut().insert(key.clone(), value.clone());
		}
		thread.set_spawned_by(self.params.spawned_by.clone());
	}

	/// The opened post of this session.
	#[cfg(feature = "action")]
	fn opened(&self) -> SessionOpened {
		SessionOpened {
			spawned_by: self.params.spawned_by.clone(),
		}
	}
}

impl Thread {
	/// Enter the thread at `entity` before its behavior runs, opening its
	/// session: reduce it, then pick the stored thread `entry` continues from
	/// its [`MountThreadStore`] and mount it.
	///
	/// A continued thread adopts the stored id and enters with its bounded
	/// window ([`ThreadStoreProvider::materialize_window`]), its stored seed
	/// standing for the one this session resolved; a created one is stamped
	/// by `entry` and keeps its resolved seed, a branch entering with its
	/// parent's history before it. Either
	/// way the session opens with a [`SessionOpened`] post. A thread with no
	/// mount persists nothing, and opens its session all the same.
	///
	/// Once per entity: the mount is consumed by the first entry. A thread
	/// entity is one session of one thread; another session is another
	/// thread entity.
	///
	/// ## Errors
	///
	/// Errors when a seed never resolved, naming it, when the mount has no
	/// store above the thread to keep its records in, or when the entry names
	/// a thread the store lacks.
	#[cfg(feature = "action")]
	pub async fn enter(
		world: AsyncWorld,
		entity: Entity,
		entry: ThreadEntry,
	) -> Result {
		let mount =
			world
				.with(
					move |world: &mut World| -> Result<
						Option<(ThreadStore, Option<u64>, Thread)>,
					> {
						ThreadWindow::reduce_now(world);
						Self::assert_reduced(world, entity)?;
						let Some(mount) =
							world.get::<MountThreadStore>(entity).cloned()
						else {
							return Ok(None);
						};
						let store = world
							.with_state::<AncestorQuery<&BlobStore>, _>(
								|query| query.get(entity).cloned(),
							)
							.map_err(|_| {
								bevyhow!(
									"no store above the thread to keep its records under `{}`",
									mount.path
								)
							})?;
						world.entity_mut(entity).remove::<MountThreadStore>();
						let fresh =
							world.get::<Thread>(entity).cloned().ok_or_else(
								|| bevyhow!("entered {entity} is no thread"),
							)?;
						(mount.build(&store), mount.tail, fresh).xsome().xok()
					},
				)
				.await?;
		let Some((store, tail, mut fresh)) = mount else {
			return world
				.with(move |world: &mut World| -> Result {
					let mut thread = world
						.get_mut::<Thread>(entity)
						.ok_or_else(|| bevyhow!("thread {entity} despawned"))?;
					entry.stamp(&mut thread);
					let id = thread.id();
					world
						.get_mut::<ThreadWindow>(entity)
						.ok_or_else(|| bevyhow!("thread {entity} despawned"))?
						.open_session(id, entry.opened())
						.map(|_| ())
				})
				.await;
		};

		// continue the stored thread the entry names, or stamp a fresh one;
		// either way its window loads before the store mounts, so the sync
		// never flushes an un-adopted thread
		store.store_try_create().await?;
		let stored = entry.find(&store).await?;
		let continued = stored.is_some();
		let record = match stored {
			Some(stored) => stored,
			None => {
				entry.stamp(&mut fresh);
				fresh
			}
		};
		let loaded = store.materialize_window(&record, tail).await?;
		world
			.with(move |world: &mut World| -> Result {
				let synced = loaded.posts().iter().map(Post::id).collect();
				let mut window = world
					.get_mut::<ThreadWindow>(entity)
					.ok_or_else(|| bevyhow!("thread {entity} despawned"))?;
				if continued {
					window.clear_posts();
				}
				window.merge_stored(loaded);
				window.open_session(record.id(), entry.opened())?;
				world.entity_mut(entity).insert((
					record,
					SyncedPosts::new(synced),
					store,
				));
				Ok(())
			})
			.await
	}

	/// Refuse a thread the reduction skipped because a [`SeedSource`] under it
	/// never landed, naming each and the error its build recorded.
	#[cfg(feature = "action")]
	fn assert_reduced(world: &mut World, entity: Entity) -> Result {
		if world.get::<ThreadWindow>(entity).is_some() {
			return Ok(());
		}
		world.with_state::<(
			Query<&Children>,
			Query<&SeedSource>,
			AncestorQuery<&TemplateError>,
		), _>(|(children, sources, errors)| {
			let pending = children
				.iter_descendants(entity)
				.filter_map(|child| sources.get(child).ok())
				.map(|source| source.label.to_string())
				.collect::<Vec<_>>();
			let cause = errors
				.get(entity)
				.map(|error| format!(": {}", error.error))
				.unwrap_or_default();
			match pending.is_empty() {
				true => bevybail!("entered {entity} is no thread"),
				false => bevybail!(
					"thread {entity} was entered before its seeds resolved ({}){cause}",
					pending.join(", ")
				),
			}
		})
	}
}

#[cfg(all(test, feature = "action"))]
mod test {
	use super::*;
	use beet_action::prelude::*;

	/// One session of a thread continuing by `chat=main` under `store`,
	/// its System actor seeded with `seeds`, closed and flushed: the
	/// thread's record and window as the session left them.
	async fn session(
		store: &BlobStore,
		seeds: &[&str],
		tail: Option<u64>,
	) -> (Thread, ThreadWindow) {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.install_type::<ThreadPlugin>();
		let root = app.world_mut().spawn(store.clone()).id();
		let thread = app
			.world_mut()
			.spawn((ChildOf(root), Thread::default(), MountThreadStore {
				path: RelPath::new("threads"),
				tail,
			}))
			.id();
		let actor = app
			.world_mut()
			.spawn((
				ChildOf(thread),
				Actor::new_with_id(
					ActorId::from_u128(1),
					"System",
					ActorKind::System,
				),
			))
			.id();
		for seed in seeds {
			app.world_mut()
				.spawn((ChildOf(actor), Post::spawn(seed.to_string())));
		}
		app.world_mut()
			.run_async_local_then(async move |world: AsyncWorld| -> Result {
				let entry =
					ThreadEntry::new(Map::new([("chat", "main")]), default());
				Thread::enter(world.clone(), thread, entry).await?;
				Thread::close(&world, thread, Outcome::PASS).await?;
				ThreadStore::flush(&world.entity(thread)).await
			})
			.await
			.unwrap();
		(
			app.world().get::<Thread>(thread).unwrap().clone(),
			app.world().get::<ThreadWindow>(thread).unwrap().clone(),
		)
	}

	/// The text of the window's seed posts and of what model input carries,
	/// in order.
	fn seeds(window: &ThreadWindow) -> (Vec<String>, Vec<String>) {
		(
			window
				.posts()
				.iter()
				.filter(|post| window.seed().contains(&post.id()))
				.map(|post| post.to_string())
				.collect(),
			window
				.model_posts(None)
				.into_iter()
				.map(|view| view.post.to_string())
				.collect(),
		)
	}

	/// A continued thread enters with its stored seed, never the one its
	/// session resolved, edited or not.
	#[beet_core::test]
	async fn a_continued_thread_keeps_its_stored_seed() {
		let store = BlobStore::temp();
		let (first, _) = session(&store, &["be brief"], None).await;
		let (thread, window) =
			session(&store, &["be thorough", "cite sources"], None).await;
		thread.id().xpect_eq(first.id());
		seeds(&window).xpect_eq((vec!["be brief".to_string()], vec![
			"be brief".to_string(),
		]));
	}

	/// A thread of more posts than its bound enters with only its tail,
	/// keeping its seed however old.
	#[beet_core::test]
	async fn a_long_thread_enters_with_its_tail() {
		let store = BlobStore::temp();
		for _ in 0..3 {
			session(&store, &["be brief"], None).await;
		}
		// three sessions of two bracket posts after the seed
		let (_, window) = session(&store, &["be brief"], Some(2)).await;
		window.posts().len().xpect_eq(5);
		window.posts()[0].to_string().xpect_eq("be brief");
		window.posts()[1..]
			.iter()
			.all(|post| post.intent().is_session())
			.xpect_true();
	}
}
