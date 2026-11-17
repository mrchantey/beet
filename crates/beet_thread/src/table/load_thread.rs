use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Declare a thread-record store from markup, persisting the thread's records
/// (its whole conversation) to a single JSON object under `path` in the store
/// at or above the thread, so a reload of the scene resumes rather than
/// starting fresh: `<Thread {(Sequence, MountThreadStore{path:"threads/chat"})}>`.
///
/// This is a *deferred* mount: [`Thread::enter`] builds the [`ThreadStore`],
/// adopts the stored conversation by seed hash, and only then inserts the
/// store onto the thread, so the persistence sync never flushes a fresh,
/// un-adopted thread (which would fork a duplicate on every reload). One path
/// keeps every thread entered under it, each found by its own seed hash, ie
/// the thread a [`ThreadRoute`] builds per request.
/// This is the thread-record store (the conversation); the store it scopes is
/// the same one an agent's tools resolve, so the records sit beside whatever
/// else the scene keeps there, under their own path.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct MountThreadStore {
	/// The directory the thread's records persist under, in the store at or
	/// above the thread.
	pub path: RelPath,
}

impl MountThreadStore {
	/// Build the [`ThreadStore`] this declares: a single-JSON-object
	/// [`BlobThreadStore`] under `path` in `store`.
	pub fn build(&self, store: &BlobStore) -> ThreadStore {
		ThreadStore::new(BlobThreadStore::new(
			store.with_subdir(self.path.clone()),
		))
	}
}

/// Governs the fate of a superseded thread when the author scene's seed is
/// edited (forking a new thread under a new seed hash).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub enum ThreadConfig {
	/// Leave superseded threads in the store (the default).
	#[default]
	Keep,
	/// Remove superseded threads from the store on fork.
	Remove,
}

impl Thread {
	/// Enter the thread at `entity` before its behavior runs: reduce it, then
	/// adopt its [`MountThreadStore`] by seed hash and mount it, so a stored
	/// conversation with the same seed resumes and any other seed starts a
	/// thread of its own. `new` discards the stored conversation first, as
	/// `--new` does.
	///
	/// Once per entity: the mount is consumed by the first entry, so a later
	/// one finds nothing to adopt. A thread entity is one thread for its whole
	/// life, its seed resolved at build ([`SeedSource`]); another seed is
	/// another thread entity.
	///
	/// ## Errors
	///
	/// Errors when a seed never resolved, naming it, or when the mount has no
	/// store above the thread to keep its records in.
	#[cfg(feature = "action")]
	pub async fn enter(world: AsyncWorld, entity: Entity, new: bool) -> Result {
		let Some(store) = world
			.with(move |world: &mut World| -> Result<Option<ThreadStore>> {
				ThreadWindow::reduce_now(world);
				Self::assert_reduced(world, entity)?;
				let Some(mount) =
					world.get::<MountThreadStore>(entity).cloned()
				else {
					return Ok(None);
				};
				let store = world
					.with_state::<AncestorQuery<&BlobStore>, _>(|query| {
						query.get(entity).cloned()
					})
					.map_err(|_| {
						bevyhow!(
							"no store above the thread to keep its records under `{}`",
							mount.path
						)
					})?;
				world.entity_mut(entity).remove::<MountThreadStore>();
				mount.build(&store).xsome().xok()
			})
			.await?
		else {
			return Ok(());
		};
		// adopt before mounting, so the sync never flushes an un-adopted
		// thread (which would fork a duplicate on every reload)
		Self::adopt(world.clone(), store.clone(), entity, new).await?;
		world
			.with(move |world: &mut World| {
				if let Ok(mut thread) = world.get_entity_mut(entity) {
					thread.insert(store);
				}
			})
			.await;
		Ok(())
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

	/// Adopt or bootstrap an already-spawned thread `entity` against `store`, keyed
	/// by its seed hash, without attaching any turn trigger.
	///
	/// The entity must carry the authored [`Thread`] plus its record store (as a
	/// component or ancestor). It is reduced (idempotently) into a [`ThreadWindow`]
	/// whose seed hash is looked up in the store: a **match** adopts that thread's id
	/// and loads its conversation into the window (behavior re-applies, history is
	/// immutable); no match **bootstraps** the seeds under a fresh id. `new`
	/// removes the matching stored thread first, so the seeds bootstrap, while
	/// every other thread in the store stays. Driving the
	/// conversation is the caller's job (eg an event-driven composer), so this
	/// suits a `.bsx`-spawned scene where the trigger would otherwise re-reply on
	/// reload. [`Thread::enter`] is the one caller in production, adopting before
	/// it mounts the store and the behavior runs.
	#[cfg(feature = "action")]
	pub async fn adopt(
		world: AsyncWorld,
		store: ThreadStore,
		entity: Entity,
		new: bool,
	) -> Result<Entity> {
		store.store_try_create().await?;

		// reduce the authored scene and read its seed hash
		let (seed_hash, config) = world
			.with(move |world: &mut World| -> Result<(u64, ThreadConfig)> {
				ThreadWindow::reduce_now(world);
				let config = world
					.get::<ThreadConfig>(entity)
					.copied()
					.unwrap_or_default();
				let window =
					world.get::<ThreadWindow>(entity).ok_or_else(|| {
						bevyhow!("spawned scene has no Thread to reduce")
					})?;
				Ok((window.seed_hash(), config))
			})
			.await?;

		// find the stored thread sharing this seed, discarding it on `new`
		let mut stored = store
			.threads()
			.await?
			.into_iter()
			.find(|thread| thread.seed_hash() == seed_hash);
		if let Some(thread) = stored.as_ref().filter(|_| new) {
			store.remove_thread(thread.id()).await?;
			stored = None;
		}
		let (stored_actors, stored_posts) = match &stored {
			Some(thread) => (
				store.actors().await?,
				store.thread_posts(thread.id(), None).await?,
			),
			None => (Vec::new(), Vec::new()),
		};

		// honor `ThreadConfig::Remove` for the superseded thread(s)
		if stored.is_none() && config == ThreadConfig::Remove {
			store.store_remove().await.ok();
			store.store_try_create().await?;
		}

		// adopt or bootstrap
		world
			.with(move |world: &mut World| -> Result {
				{
					let mut thread = world
						.get_mut::<Thread>(entity)
						.ok_or_else(|| bevyhow!("thread entity despawned"))?;
					thread.set_seed_hash(seed_hash);
					if let Some(stored) = &stored {
						thread.set_id(stored.id());
					}
				}
				if stored.is_some() {
					let synced =
						stored_posts.iter().map(|post| post.id()).collect();
					world
						.get_mut::<ThreadWindow>(entity)
						.ok_or_else(|| bevyhow!("thread window despawned"))?
						.load_records(stored_actors, stored_posts);
					world.entity_mut(entity).insert(SyncedPosts::new(synced));
				}
				Ok(())
			})
			.await
			.map(|_| entity)
	}
}

#[cfg(all(test, feature = "action"))]
mod test {
	use super::*;
	use beet_action::prelude::*;

	/// A persisted author scene: pinned actor ids, one user seed, one mock agent.
	fn scene(user_id: ActorId, agent_id: ActorId, prompt: &str) -> impl Bundle {
		(Thread::default(), Sequence::new(), children![
			(
				Actor::new_with_id(user_id, "User", ActorKind::User),
				children![Post::spawn(prompt.to_string())],
			),
			(
				Actor::new_with_id(agent_id, "Agent", ActorKind::Agent),
				MockPostStreamer::default(),
			),
		])
	}

	/// The seed hash is stable for an unchanged seed and forks when it changes.
	#[beet_core::test]
	fn seed_hash_stable_and_forks() {
		let user = ActorId::new_now();
		let agent = ActorId::new_now();
		let hash = |prompt: &str| {
			let mut world = World::new();
			let thread = world.spawn(scene(user, agent, prompt)).id();
			world.flush();
			ThreadWindow::reduce_now(&mut world);
			world.get::<ThreadWindow>(thread).unwrap().seed_hash()
		};
		// same seed -> same hash, behavior aside
		hash("hi").xpect_eq(hash("hi"));
		// edited seed -> a fork
		(hash("hi") != hash("howdy")).xpect_true();
	}

	/// Spawn the scene, adopt it against `backing`, mount the store and run one
	/// turn: the sequence [`RunThread`]'s entry point performs, against an
	/// in-memory store.
	fn drive_load(backing: &BlobThreadStore, user: ActorId, agent: ActorId) {
		let store = ThreadStore::new(backing.clone());
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.install_type::<ThreadPlugin>();
		app.world_mut()
			.with_state::<AsyncCommands, _>(move |commands| {
				commands.run(async move |world: AsyncWorld| -> Result {
					let thread = world
						.with(move |world: &mut World| {
							world.spawn(scene(user, agent, "hi")).id()
						})
						.await;
					// adopt before mounting, so the sync never flushes an
					// un-adopted thread
					Thread::adopt(world.clone(), store.clone(), thread, false)
						.await?;
					world
						.with(move |world: &mut World| {
							world.entity_mut(thread).insert(store);
						})
						.await;
					world.entity(thread).call::<(), Outcome>(()).await?;
					Ok(())
				});
			});
		for _ in 0..80 {
			app.update();
		}
	}

	/// Bootstrapping then reloading the same seed continues the stored thread
	/// (adopts its id) rather than forking a new one, and runs behavior against
	/// the loaded history.
	#[beet_core::test]
	async fn bootstrap_then_reload_adopts_thread() {
		let backing = BlobThreadStore::temp();
		let user = ActorId::new_now();
		let agent = ActorId::new_now();

		// bootstrap: a fresh thread persists its seed + first reply
		drive_load(&backing, user, agent);
		let threads = backing.threads().await.unwrap();
		threads.len().xpect_eq(1);
		let thread_id = threads[0].id();
		let bootstrap_posts =
			backing.thread_posts(thread_id, None).await.unwrap();
		bootstrap_posts
			.iter()
			.any(|post| post.body_str().ok() == Some("you said: hi"))
			.xpect_true();

		// reload: same seed -> same thread id, more posts appended
		drive_load(&backing, user, agent);
		let threads = backing.threads().await.unwrap();
		threads.len().xpect_eq(1);
		threads[0].id().xpect_eq(thread_id);
		let reload_posts = backing.thread_posts(thread_id, None).await.unwrap();
		(reload_posts.len() > bootstrap_posts.len()).xpect_true();
	}
}
