use crate::prelude::*;
#[cfg(feature = "action")]
use beet_action::prelude::*;
use beet_core::prelude::*;

/// A window's records: the actors, whole history and response metadata of
/// the stored thread it is open onto, held in memory.
///
/// A copy of the [`ThreadStore`], never a truth of its own, loaded whole: the
/// window's behavior streams into it, its view projects it, each model call
/// derives its input from it ([`Self::model_posts`]) and the feed carries
/// each change to the thread's other windows ([`ThreadFeed`]).
///
/// Author resolution is in memory (`records.actor(post.author())`), with no
/// entity walk; ordering is by [`PostId`] (a time-sortable [`Uuid7`]).
#[derive(Debug, Default, Clone, Component)]
pub struct ThreadRecords {
	/// All actors that appear in the posts of these records, keyed by [`ActorId`].
	actors: HashMap<ActorId, Actor>,
	/// Ordered list of posts (by [`PostId`]).
	posts: Vec<Post>,
	/// Per-post response metadata, used for `previous_response_id` chaining.
	metas: HashMap<PostId, ResponseMeta>,
	/// The posts pushed or updated here since the feed last took them, each
	/// once, in the order they changed.
	fed: Vec<PostId>,
}

impl ThreadRecords {
	pub fn new() -> Self { Self::default() }

	// ── actors ──────────────────────────────────────────────────────────

	pub fn actors(&self) -> &HashMap<ActorId, Actor> { &self.actors }

	/// Resolve an actor by id, in memory.
	#[track_caller]
	pub fn actor(&self, id: ActorId) -> Result<&Actor> {
		self.actors
			.get(&id)
			.ok_or_else(|| bevyhow!("No actor {id} in records"))
	}

	/// Insert (or replace) an actor in the records.
	pub fn insert_actor(&mut self, actor: Actor) -> ActorId {
		let id = actor.id();
		self.actors.insert(id, actor);
		id
	}

	// ── posts ───────────────────────────────────────────────────────────

	pub fn posts(&self) -> &[Post] { &self.posts }
	pub fn is_empty(&self) -> bool { self.posts.is_empty() }
	pub fn last_post(&self) -> Option<&Post> { self.posts.last() }

	/// Append a post, or replace the existing post with the same id, for the
	/// feed to carry to the thread's other windows. New posts keep insertion
	/// (ie [`PostId`]) order; modified posts update in place.
	pub fn upsert_post(&mut self, post: Post) {
		self.feed(post.id());
		self.upsert_quiet(post);
	}

	/// Upsert `post` without feeding it.
	fn upsert_quiet(&mut self, post: Post) {
		match self
			.posts
			.iter_mut()
			.find(|existing| existing.id() == post.id())
		{
			Some(existing) => *existing = post,
			None => self.posts.push(post),
		}
	}

	/// Mark `id` changed, for the feed.
	fn feed(&mut self, id: PostId) {
		if !self.fed.contains(&id) {
			self.fed.push(id);
		}
	}

	/// Join each post with its authoring [`Actor`] from the records.
	pub fn authored_posts(&self) -> impl Iterator<Item = AuthoredPost<'_>> {
		self.posts.iter().filter_map(|post| {
			self.actors
				.get(&post.author())
				.map(|actor| AuthoredPost { post, actor })
		})
	}

	/// The authored posts after an exclusive cursor. A missing or unknown cursor
	/// returns them all (the caller has seen none of these posts).
	pub fn posts_after(&self, after: Option<PostId>) -> Vec<AuthoredPost<'_>> {
		let views = self.authored_posts().collect::<Vec<_>>();
		let Some(after) = after else { return views };
		match views.iter().position(|view| view.post.id() == after) {
			Some(i) => views[i + 1..].to_vec(),
			None => views,
		}
	}

	/// The posts a model reads, after an exclusive cursor: every authored
	/// post but the [`PostIntent::Log`] posts, which stay in the records, the
	/// record and the view.
	pub fn model_posts(&self, after: Option<PostId>) -> Vec<AuthoredPost<'_>> {
		self.posts_after(after)
			.into_iter()
			.filter(|view| view.post.intent() != PostIntent::Log)
			.collect()
	}

	/// Merge stored records under these: the stored posts and metadata,
	/// then every actor of these records over the stored ones, so
	/// the authored definitions win while the stored authors still resolve.
	pub fn merge_stored(&mut self, stored: ThreadRecords) {
		let ThreadRecords {
			actors,
			posts,
			metas,
			fed: _,
		} = stored;
		for (id, actor) in actors {
			self.actors.entry(id).or_insert(actor);
		}
		for post in posts {
			self.upsert_quiet(post);
		}
		self.metas.extend(metas);
		self.posts.sort_by_key(|post| post.id());
	}

	/// Take what changed since the feed last took it: each changed post as it
	/// stands, its author and its metadata. Empty when nothing changed.
	pub fn take_feed(&mut self) -> ThreadFeed {
		let mut feed = ThreadFeed::default();
		for id in core::mem::take(&mut self.fed) {
			let Some(post) = self.posts.iter().find(|post| post.id() == id)
			else {
				continue;
			};
			if let Some(actor) = self.actors.get(&post.author())
				&& !feed.actors.iter().any(|known| known.id() == actor.id())
			{
				feed.actors.push(actor.clone());
			}
			if let Some(meta) = self.metas.get(&id) {
				feed.metas.push(meta.clone());
			}
			feed.posts.push(post.clone());
		}
		feed
	}

	/// Whether anything changed since the feed last took it.
	pub fn has_feed(&self) -> bool { !self.fed.is_empty() }

	/// Apply `feed`, another window's changes, in [`PostId`] order and
	/// without feeding them again: an actor these records lack is added, the
	/// rest are replaced.
	pub fn receive(&mut self, feed: &ThreadFeed) {
		for actor in &feed.actors {
			self.actors
				.entry(actor.id())
				.or_insert_with(|| actor.clone());
		}
		for post in &feed.posts {
			self.upsert_quiet(post.clone());
		}
		for meta in &feed.metas {
			self.metas.insert(meta.post_id, meta.clone());
		}
		self.posts.sort_by_key(|post| post.id());
	}

	/// Every record here as a feed, for a window joining the thread.
	pub fn as_feed(&self) -> ThreadFeed {
		ThreadFeed {
			actors: self.actors.values().cloned().collect(),
			posts: self.posts.clone(),
			metas: self.metas.values().cloned().collect(),
		}
	}

	// ── response metadata ───────────────────────────────────────────────

	/// Set the metadata of its post, for the feed to carry.
	pub fn set_meta(&mut self, meta: ResponseMeta) {
		self.feed(meta.post_id);
		self.metas.insert(meta.post_id, meta);
	}
	pub fn meta(&self, post_id: PostId) -> Option<&ResponseMeta> {
		self.metas.get(&post_id)
	}
	pub fn metas(&self) -> impl Iterator<Item = &ResponseMeta> {
		self.metas.values()
	}

	/// The most recent post authored by `agent_id` whose stored [`ResponseMeta`]
	/// matches the provider and model, for `previous_response_id` patterns.
	pub fn stored_response(
		&self,
		agent_id: ActorId,
		provider_slug: &str,
		model_slug: &str,
	) -> Option<(PostId, &ResponseMeta)> {
		self.posts.iter().rev().find_map(|post| {
			if post.author() != agent_id {
				return None;
			}
			let meta = self.metas.get(&post.id())?;
			(meta.provider_slug == provider_slug
				&& meta.model_slug == model_slug
				&& meta.response_stored)
				.then_some((post.id(), meta))
		})
	}

	/// Bound the image bytes of these records: replace every image post older than the
	/// most recent `keep` with a short text stub, in place (same [`PostId`] and
	/// position).
	///
	/// The records stay append-only, which is the point: no post is ever dropped,
	/// so the provider's prompt-cache prefix stays stable and only the last few
	/// turns churn. Image posts are oldest-first (posts are id-ordered) and a
	/// given image is stubbed exactly once, so the prefix grows with the
	/// conversation rather than being rewritten by each call.
	///
	/// Call between turns, once the turn's new image has landed. [`StubOldImages`]
	/// is the standalone action form.
	pub fn stub_old_images(&mut self, keep: usize) {
		let stub_count = self
			.posts
			.iter()
			.filter(|post| post.media_type().is_image())
			.count()
			.saturating_sub(keep);
		if stub_count == 0 {
			return;
		}
		for post in self
			.posts
			.iter_mut()
			.filter(|post| post.media_type().is_image())
			.take(stub_count)
		{
			post.set_media_type(MediaType::Text);
			post.set_text(STUBBED_IMAGE);
			if !self.fed.contains(&post.id()) {
				self.fed.push(post.id());
			}
		}
	}
}

/// One window's changes to its records, carried to every other window of
/// the same stored thread ([`ThreadRecords::take_feed`]).
#[derive(Debug, Default, Clone)]
pub struct ThreadFeed {
	/// The authors of [`Self::posts`].
	pub actors: Vec<Actor>,
	/// Each post as it stands, a post in progress included.
	pub posts: Vec<Post>,
	/// The metadata of [`Self::posts`].
	pub metas: Vec<ResponseMeta>,
}

impl ThreadFeed {
	pub fn is_empty(&self) -> bool { self.posts.is_empty() }
}

/// The text a stubbed-out older image is replaced with by
/// [`ThreadRecords::stub_old_images`].
const STUBBED_IMAGE: &str = "[earlier image, no longer shown]";

// ═══════════════════════════════════════════════════════════════════════
// AuthoredPost
// ═══════════════════════════════════════════════════════════════════════

/// A post joined with its authoring [`Actor`], sourced from a [`ThreadRecords`].
#[derive(Debug, Clone, Copy)]
pub struct AuthoredPost<'a> {
	pub post: &'a Post,
	pub actor: &'a Actor,
}

impl std::ops::Deref for AuthoredPost<'_> {
	type Target = Post;
	fn deref(&self) -> &Self::Target { self.post }
}

impl AuthoredPost<'_> {
	pub fn actor_id(&self) -> ActorId { self.actor.id() }

	/// Wraps text in XML metadata tags so models can distinguish speakers.
	/// Used for non-assistant, non-system, non-developer messages.
	pub fn wrap_user_text(&self, text: &str) -> String {
		format!(
			"<post author={} author_kind={} author_id={}>{}</post>",
			self.actor.name(),
			self.actor.kind().input_str(),
			self.actor.id(),
			text
		)
	}
}

// ═══════════════════════════════════════════════════════════════════════
// ActorRef
// ═══════════════════════════════════════════════════════════════════════

/// A behavior entity's reference to the [`Actor`] it acts as. The actor's full
/// definition lives in the [`ThreadRecords`]; the behavior scene carries only
/// this id plus the actor's behavior (its [`Action`] and tools).
///
/// Produced by the author-to-behavior reduction, which swaps every in-place
/// `Actor` for an `ActorRef`.
#[derive(Debug, Clone, Copy, Deref, Component, Reflect)]
#[reflect(Component)]
pub struct ActorRef(pub ActorId);

// ═══════════════════════════════════════════════════════════════════════
// SeedPost
// ═══════════════════════════════════════════════════════════════════════

/// An author-time seed post: an unresolved [`IntoPost`] payload spawned as a
/// child of its `<CreateActor>`, awaiting reduction.
///
/// This is the runtime component the author-to-behavior reduction queries
/// ([`ThreadRecords::reduce`]): it resolves the seed's author (its actor parent)
/// and thread, hoists the materialized [`Post`] into the [`ThreadRecords`], and
/// despawns the entity.
///
/// It is the payload, not the tag. `<CreatePost>` is the `#[template]` that
/// produces a `SeedPost` from markup; the two cannot be merged: a `#[template]`
/// expands away at build and leaves no component for the runtime `Query<&SeedPost>`
/// to read, and `content: IntoPost` is not attribute-coercible so `SeedPost`
/// itself cannot be a tag. The template adapts a coercible `text` into `IntoPost`;
/// this component is what the world stores and reduces. (Mirrors the
/// `<CreateActor>`/[`Actor`] tag/payload pair.) Authored via [`Post::spawn`] /
/// `<CreatePost>`.
#[derive(Debug, Clone, Component)]
pub struct SeedPost {
	pub content: IntoPost,
	pub intent: PostIntent,
	pub status: PostStatus,
}

impl SeedPost {
	/// Resolve into a [`Post`] for the given author and thread.
	pub fn into_post(self, author: ActorId, thread: ThreadId) -> Post {
		let mut post = self.content.into_post(author, thread);
		post.set_intent(self.intent);
		post.set_status(self.status);
		post
	}
}

// ═══════════════════════════════════════════════════════════════════════
// The reduction: author scene -> ThreadRecords + behavior scene
// ═══════════════════════════════════════════════════════════════════════

impl ThreadRecords {
	/// Reduce every freshly-spawned authored [`Thread`] into a [`ThreadRecords`] plus
	/// a lean behavior scene, keeping its seed posts.
	///
	/// For each thread lacking records, walk its descendant actors in author order
	/// and: hoist each [`Actor`] into `records.actors`, hoist each [`SeedPost`] into
	/// `records.posts`, then rewrite the actor entity. An actor that carries behavior
	/// (an [`Action`]) becomes an [`ActorRef`] keeping its behavior and tools; a
	/// seed-only actor (eg the system prompt) is despawned, leaving no turn behind.
	///
	/// A thread mounting a store ([`MountThreadStore`]) or holding a
	/// [`SeedSource`] waits for its entry, the one place that knows whether
	/// its seed is kept ([`Self::reduce_entered`]).
	///
	/// Idempotent: gated on `Without<ThreadRecords>`, so it runs once per thread.
	pub fn reduce(
		mut commands: Commands,
		threads: Query<
			(Entity, &Thread),
			(Without<ThreadRecords>, Without<MountThreadStore>),
		>,
		reduction: Reduction,
		#[cfg(feature = "action")] sources: Query<(), With<SeedSource>>,
	) -> Result {
		for (entity, thread) in threads.iter() {
			#[cfg(feature = "action")]
			if reduction
				.children
				.iter_descendants(entity)
				.any(|child| sources.contains(child))
			{
				continue;
			}
			reduction.reduce(&mut commands, entity, thread.id(), true)?;
		}
		Ok(())
	}

	/// Reduce the thread at `entity` as its entry decides, consuming its
	/// [`MountThreadStore`]: keeping its seed posts when the entry creates
	/// its thread, else despawning them and every [`SeedSource`] unread. A
	/// thread already reduced, which mounts no store, is left as it is.
	#[cfg(feature = "action")]
	pub(crate) fn reduce_entered(
		world: &mut World,
		entity: Entity,
		keep_seeds: bool,
	) -> Result {
		world.entity_mut(entity).remove::<MountThreadStore>();
		if world.get::<ThreadRecords>(entity).is_some() {
			return Ok(());
		}
		world.run_system_cached_with::<_, Result, _, _>(
			Self::reduce_one,
			(entity, keep_seeds),
		)??;
		world.flush();
		Ok(())
	}

	/// Reduce the one thread at `entity`, keeping or dropping its seeds.
	#[cfg(feature = "action")]
	fn reduce_one(
		In((entity, keep_seeds)): In<(Entity, bool)>,
		mut commands: Commands,
		threads: Query<&Thread>,
		reduction: Reduction,
	) -> Result {
		let thread = threads.get(entity)?.id();
		reduction.reduce(&mut commands, entity, thread, keep_seeds)
	}

	/// Run [`ThreadRecords::reduce`] immediately and flush, for manual consumers that
	/// pump the world directly (eg `Post::run_oneshot`, tests) rather than the schedule.
	pub fn reduce_now(world: &mut World) {
		let _ = world.run_system_cached::<Result, _, _>(Self::reduce);
		world.flush();
	}

	/// Spawn an authored thread `scene` and reduce it into a [`ThreadRecords`] plus
	/// behavior scene in one step, returning the root entity. The behavior is *not*
	/// triggered; the caller drives it (eg `entity.call(())`), so the records are
	/// always populated before any turn runs.
	pub async fn spawn_reduced(
		world: AsyncWorld,
		scene: impl Bundle,
	) -> Result<Entity> {
		world
			.with(move |world: &mut World| {
				let root = world.spawn(scene).id();
				Self::reduce_now(world);
				root
			})
			.await
			.xok()
	}
}

// ═══════════════════════════════════════════════════════════════════════
// Reduction: the walk over one authored thread
// ═══════════════════════════════════════════════════════════════════════

/// The queries reducing one authored thread ([`ThreadRecords::reduce`]).
#[derive(SystemParam)]
pub struct Reduction<'w, 's> {
	children: Query<'w, 's, &'static Children>,
	actors: Query<'w, 's, &'static Actor>,
	seeds: Query<'w, 's, &'static SeedPost>,
	#[cfg(feature = "action")]
	sources: Query<'w, 's, (), With<SeedSource>>,
	#[cfg(feature = "action")]
	behaviors: Query<'w, 's, (), With<ActionMeta>>,
}

impl Reduction<'_, '_> {
	/// Reduce the thread at `entity`, its posts belonging to `thread`,
	/// keeping or dropping its seed posts.
	fn reduce(
		&self,
		commands: &mut Commands,
		entity: Entity,
		thread: ThreadId,
		keep_seeds: bool,
	) -> Result {
		let mut records = ThreadRecords::new();

		// descendant actor entities, in author order
		let actor_entities = self
			.children
			.iter_descendants_depth_first(entity)
			.filter(|child| self.actors.contains(*child))
			.collect::<Vec<_>>();

		for actor_entity in actor_entities {
			let actor = self.actors.get(actor_entity)?.clone();
			let actor_id = records.insert_actor(actor);

			// hoist or drop this actor's seed posts, despawning their entities.
			// Scan all descendants (not just direct children) so seeds authored
			// through a markup `<Slot/>` are still found; actors never nest, so
			// every seed under an actor entity is unambiguously its own.
			for child in
				self.children.iter_descendants_depth_first(actor_entity)
			{
				if let Ok(seed_post) = self.seeds.get(child) {
					if keep_seeds {
						records.upsert_post(
							seed_post.clone().into_post(actor_id, thread),
						);
					}
					commands.entity(child).despawn();
				}
				#[cfg(feature = "action")]
				if !keep_seeds && self.sources.contains(child) {
					commands.entity(child).despawn();
				}
			}

			// rewrite the actor: behavior -> ActorRef, seed-only -> despawn
			#[cfg(feature = "action")]
			let has_behavior = self.behaviors.contains(actor_entity);
			#[cfg(not(feature = "action"))]
			let has_behavior = true;
			if has_behavior {
				commands
					.entity(actor_entity)
					.remove::<Actor>()
					.insert(ActorRef(actor_id));
			} else {
				commands.entity(actor_entity).despawn();
			}
		}

		records.posts.sort_by_key(|post| post.id());
		commands.entity(entity).insert(records);
		Ok(())
	}
}

// ═══════════════════════════════════════════════════════════════════════
// ThreadRecordsQuery: mutate the thread records from a behavior entity
// ═══════════════════════════════════════════════════════════════════════

/// Locate and mutate the [`ThreadRecords`] of the thread a behavior entity
/// belongs to. Behavior entities ([`ActorRef`]) are descendants of their
/// thread; the records live on the thread.
#[derive(SystemParam)]
pub struct ThreadRecordsQuery<'w, 's> {
	ancestors: Query<'w, 's, &'static ChildOf>,
	actor_refs: Query<'w, 's, &'static ActorRef>,
	thread_records:
		Query<'w, 's, (&'static Thread, &'static mut ThreadRecords)>,
	synced: Query<'w, 's, &'static mut SyncedPosts>,
}

impl ThreadRecordsQuery<'_, '_> {
	/// Walk ancestors to the thread entity carrying the [`ThreadRecords`].
	pub fn thread_entity(&self, entity: Entity) -> Result<Entity> {
		self.ancestors
			.iter_ancestors_inclusive(entity)
			.find(|ancestor| self.thread_records.contains(*ancestor))
			.ok_or_else(|| {
				bevyhow!("No ThreadRecords in ancestors of {entity}")
			})
	}

	/// The [`ActorId`] a behavior entity acts as.
	pub fn actor_id(&self, entity: Entity) -> Result<ActorId> {
		self.actor_refs
			.get(entity)
			.map(|actor_ref| **actor_ref)
			.map_err(|_| bevyhow!("entity {entity} has no ActorRef"))
	}

	/// The [`ThreadId`] of the thread a behavior entity belongs to.
	pub fn thread_id(&self, entity: Entity) -> Result<ThreadId> {
		let thread_entity = self.thread_entity(entity)?;
		Ok(self.thread_records.get(thread_entity)?.0.id())
	}

	/// Mutable access to the thread's records.
	pub fn records_mut(
		&mut self,
		entity: Entity,
	) -> Result<Mut<'_, ThreadRecords>> {
		let thread_entity = self.thread_entity(entity)?;
		Ok(self.thread_records.get_mut(thread_entity)?.1)
	}

	/// Append a post to the thread's records.
	pub fn push_post(&mut self, entity: Entity, post: Post) -> Result {
		self.records_mut(entity)?.upsert_post(post);
		Ok(())
	}

	/// Record a response's `usage` on the meta of `post`, its last post,
	/// which the next flush writes again even if it already went.
	pub fn set_usage(
		&mut self,
		entity: Entity,
		post: PostId,
		usage: Usage,
	) -> Result {
		let thread_entity = self.thread_entity(entity)?;
		let mut records = self.thread_records.get_mut(thread_entity)?.1;
		let Some(mut meta) = records.meta(post).cloned() else {
			return Ok(());
		};
		meta.usage = Some(usage);
		records.set_meta(meta);
		if let Ok(mut synced) = self.synced.get_mut(thread_entity) {
			synced.remove(post);
		}
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use super::STUBBED_IMAGE;
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The reduction hoists every actor and seed post into the records, despawns
	/// seed-only actors, and rewrites behavior actors to id-referencing entities.
	#[beet_core::test]
	fn reduction_hoists_and_rewrites() {
		let mut world = World::new();
		let thread = world
			.spawn((Thread::default(), children![
				// seed-only actor: hoisted then despawned
				(Actor::system(), children![Post::spawn("sys prompt")]),
				// behavior actor: kept as an ActorRef
				(Actor::agent(), MockPostStreamer::default()),
			]))
			.id();
		world.flush();
		ThreadRecords::reduce_now(&mut world);

		// the records hold both actors and the one seed post, author resolvable
		let records = world.get::<ThreadRecords>(thread).unwrap();
		records.actors().len().xpect_eq(2);
		records.posts().len().xpect_eq(1);
		records.posts()[0]
			.to_string()
			.xpect_eq("sys prompt".to_string());
		let author = records.posts()[0].author();
		records
			.actor(author)
			.unwrap()
			.kind()
			.xpect_eq(ActorKind::System);

		// behavior scene: exactly one ActorRef (the agent), no Actor or SeedPost
		// entities survive, and the survivor keeps its streamer behavior
		world.query::<&ActorRef>().iter(&world).count().xpect_eq(1);
		world.query::<&Actor>().iter(&world).count().xpect_eq(0);
		world.query::<&SeedPost>().iter(&world).count().xpect_eq(0);
		world
			.query_filtered::<Entity, (With<ActorRef>, With<MockPostStreamer>)>(
			)
			.iter(&world)
			.count()
			.xpect_eq(1);
	}

	/// Bounding replaces older images with a text stub in place: the post count
	/// and every id are untouched (the records stay append-only), only the newest
	/// `keep` images still carry bytes.
	#[beet_core::test]
	fn stub_old_images_bounds_the_records() {
		let thread = ThreadId::new_now();
		let user = ActorId::new_now();
		let mut records = ThreadRecords::new();
		records.insert_actor(Actor::new_with_id(user, "Cam", ActorKind::User));
		for cycle in 0..3 {
			records.upsert_post(AgentPost::new_bytes(
				user,
				thread,
				MediaType::Jpeg,
				vec![cycle as u8],
				None,
				PostStatus::Completed,
			));
		}
		let ids = records.posts().iter().map(Post::id).collect::<Vec<_>>();

		records.stub_old_images(1);

		// nothing dropped, nothing reordered: only the media types changed
		records.posts().len().xpect_eq(3);
		records
			.posts()
			.iter()
			.map(Post::id)
			.collect::<Vec<_>>()
			.xpect_eq(ids);
		records
			.posts()
			.iter()
			.filter(|post| post.media_type().is_image())
			.count()
			.xpect_eq(1);
		// the surviving image is the newest, the stubbed ones carry the stub text
		records.posts()[2].media_type().is_image().xpect_true();
		records.posts()[0]
			.to_string()
			.xpect_eq(STUBBED_IMAGE.to_string());

		// stubbing again is idempotent: a given image is only ever stubbed once
		records.stub_old_images(1);
		records
			.posts()
			.iter()
			.filter(|post| post.media_type().is_image())
			.count()
			.xpect_eq(1);
	}

	/// Streaming-shaped records mutation: upsert appends new posts and replaces
	/// existing ones in place, ordered by id.
	#[beet_core::test]
	fn upsert_appends_then_replaces() {
		let thread = ThreadId::new_now();
		let author = ActorId::new_now();
		let mut records = ThreadRecords::new();
		records.insert_actor(Actor::new_with_id(author, "A", ActorKind::Agent));

		let mut post =
			AgentPost::new_text(author, thread, "hel", PostStatus::InProgress);
		records.upsert_post(post.clone());
		records.posts().len().xpect_eq(1);

		// same id streamed further: replaced in place, not appended
		post.set_text("hello");
		post.set_status(PostStatus::Completed);
		records.upsert_post(post);
		records.posts().len().xpect_eq(1);
		records
			.last_post()
			.unwrap()
			.to_string()
			.xpect_eq("hello".to_string());
		records.last_post().unwrap().in_progress().xpect_false();
	}
}
