use crate::prelude::*;
#[cfg(feature = "action")]
use beet_action::prelude::*;
use beet_core::prelude::*;

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

/// Adopt or bootstrap an already-spawned thread `entity` against `store`, keyed
/// by its seed hash, without attaching any turn trigger.
///
/// The entity must carry the authored [`Thread`] plus its record store (as a
/// component or ancestor). It is reduced (idempotently) into a [`ThreadWindow`]
/// whose seed hash is looked up in the store: a **match** adopts that thread's id
/// and loads its conversation into the window (behavior re-applies, history is
/// immutable); no match **bootstraps** the seeds under a fresh id. Driving the
/// conversation is the caller's job (eg an event-driven composer), so this
/// suits a `.bsx`-spawned scene where the trigger would otherwise re-reply on
/// reload. See [`load_thread`] for the spawn-and-run convenience.
#[cfg(feature = "action")]
pub async fn adopt_thread(
	world: AsyncWorld,
	store: ThreadStore,
	entity: Entity,
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

	// find the stored thread sharing this seed
	let stored = store
		.threads()
		.await?
		.into_iter()
		.find(|thread| thread.seed_hash() == seed_hash);
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

/// Spawn an authored bundle `scene` with its record `store`, [`adopt_thread`] it,
/// then attach the behavior trigger ([`CallOnSpawn`]) so the turn runs once the
/// window is correct. The trigger never fires against an empty or stale window
/// because it lands only after adoption, no `Hydrating` marker required.
///
/// Topology-agnostic: the `Thread` may be the spawned root or nested under a loop
/// (eg `Repeat[Thread+Sequence]` for an interactive chat). The store mounts on
/// the `Thread` entity, where the persistence sync reads it; the kick lands on
/// the root (the loop, or the `Thread` itself when flat). Returns the `Thread`.
#[cfg(feature = "action")]
pub async fn load_thread(
	world: AsyncWorld,
	store: ThreadStore,
	scene: impl Bundle,
) -> Result<Entity> {
	let store_component = store.clone();
	// spawn + reduce, then mount the store on the Thread entity (root or nested)
	let (root, thread) = world
		.with(move |world: &mut World| -> Result<(Entity, Entity)> {
			let root = world.spawn(scene).id();
			ThreadWindow::reduce_now(world);
			let thread = thread_entity_under(world, root)?;
			world.entity_mut(thread).insert(store_component);
			Ok((root, thread))
		})
		.await?;
	adopt_thread(world.clone(), store, thread).await?;
	// kick the root (the loop, or the Thread itself) now the window is correct
	world
		.with(move |world: &mut World| {
			world
				.entity_mut(root)
				.insert(CallOnSpawn::<(), Outcome>::new(()));
		})
		.await;
	Ok(thread)
}

/// The [`Thread`] entity at or under `root`: the root itself, or nested beneath a
/// loop wrapper (eg `Repeat`).
#[cfg(feature = "action")]
fn thread_entity_under(world: &mut World, root: Entity) -> Result<Entity> {
	world
		.with_state::<(Query<(), With<Thread>>, Query<&Children>), _>(
			|(threads, children)| {
				std::iter::once(root)
					.chain(children.iter_descendants(root))
					.find(|entity| threads.contains(*entity))
			},
		)
		.ok_or_else(|| bevyhow!("spawned scene has no Thread"))
}

#[cfg(all(test, feature = "action"))]
mod test {
	use super::*;

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

	/// Drive `load_thread` to completion against a store, returning the app so
	/// the caller can keep pumping if needed.
	fn drive_load(backing: &BlobThreadStore, user: ActorId, agent: ActorId) {
		let store = ThreadStore::new(backing.clone());
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_plugin::<ThreadPlugin>();
		app.world_mut()
			.with_state::<AsyncCommands, _>(move |commands| {
				commands.run(async move |world: AsyncWorld| -> Result {
					load_thread(world, store, scene(user, agent, "hi")).await?;
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
