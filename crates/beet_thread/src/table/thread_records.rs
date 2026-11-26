use crate::prelude::*;
#[cfg(feature = "action")]
use beet_action::prelude::*;
use beet_core::prelude::*;

/// A window's records: the actors, whole history and response metadata of
/// the stored thread it is open onto, held in memory.
///
/// A copy of the [`ThreadStore`], never a truth of its own, loaded whole: the
/// window's behavior streams into it, its view projects it, each model call
/// derives its input from it ([`Self::model_posts`]) and window sync applies
/// each change to the thread's other windows ([`ThreadChanges`]).
///
/// Author resolution is in memory (`records.actor(post.author())`), with no
/// entity walk; ordering is by [`PostId`] (a time-sortable [`Uuid7`]).
#[derive(Debug, Default, Clone, Component)]
pub struct ThreadRecords {
	/// The actors the window's scene authors and those that authored a post,
	/// keyed by [`ActorId`].
	actors: HashMap<ActorId, Actor>,
	/// Ordered list of posts (by [`PostId`]).
	posts: Vec<Post>,
	/// Per-post response metadata, used for `previous_response_id` chaining.
	metas: HashMap<PostId, ResponseMeta>,
	/// The posts pushed or updated here since window sync last took them, each
	/// once, in the order they changed.
	changed: Vec<PostId>,
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

	/// The author of a post naming `actor` or none: the named actor, else the
	/// thread's first actor of kind User by id.
	///
	/// ## Errors
	///
	/// Errors when the named actor is not in the records, or none is named
	/// and the records hold no User actor.
	pub fn author(&self, actor: Option<ActorId>) -> Result<ActorId> {
		match actor {
			Some(actor) => self.actor(actor)?.id().xok(),
			None => self
				.actors
				.values()
				.filter(|actor| actor.kind() == ActorKind::User)
				.map(Actor::id)
				.min()
				.ok_or_else(|| {
					bevyhow!(
						"no actor is named to author the post, and the thread holds no User actor"
					)
				}),
		}
	}

	// ── posts ───────────────────────────────────────────────────────────

	pub fn posts(&self) -> &[Post] { &self.posts }
	pub fn is_empty(&self) -> bool { self.posts.is_empty() }
	pub fn last_post(&self) -> Option<&Post> { self.posts.last() }

	/// Append a post, or replace the existing post with the same id, for window
	/// sync to apply to the thread's other windows. New posts keep insertion
	/// (ie [`PostId`]) order; modified posts update in place.
	pub fn upsert_post(&mut self, post: Post) {
		self.mark_changed(post.id());
		self.upsert_quiet(post);
	}

	/// Upsert `post` without marking it changed.
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

	/// Mark `id` changed, for window sync.
	fn mark_changed(&mut self, id: PostId) {
		if !self.changed.contains(&id) {
			self.changed.push(id);
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

	/// Every authored post that is not a [`PostIntent::Log`]: what was said,
	/// rather than what happened.
	pub fn content_posts(&self) -> impl Iterator<Item = AuthoredPost<'_>> {
		self.authored_posts()
			.filter(|view| view.post.intent() != PostIntent::Log)
	}

	/// The newest post that is not a [`PostIntent::Log`].
	pub fn last_content_post(&self) -> Option<&Post> {
		self.posts
			.iter()
			.rev()
			.find(|post| post.intent() != PostIntent::Log)
	}

	/// The thread's last word: the newest post that is neither a
	/// [`PostIntent::Log`] nor the output of a call an interrupt closed, which
	/// nobody said. A note that cut a tool call is the last word, though the
	/// output closing that call was written after it.
	pub fn last_word(&self) -> Option<&Post> {
		self.posts.iter().rev().find(|post| {
			let closed_call = post.status() == PostStatus::Interrupted
				&& AgentPost::new(post)
					.is_some_and(|post| post.is_function_call_output());
			post.intent() != PostIntent::Log && !closed_call
		})
	}

	/// What a model reads, after an exclusive cursor, in model order (a
	/// function call followed at once by its output): every authored post that is not a log, and
	/// each run of joins between them as one note ([`Self::join_note`]). Any
	/// other log kind stays out, so it never splits a run.
	pub fn model_posts(&self, after: Option<PostId>) -> Vec<ModelPost<'_>> {
		let ordered = self.model_order();
		let start = after
			.and_then(|after| {
				ordered.iter().position(|view| view.post.id() == after)
			})
			.map_or(0, |index| index + 1);
		let mut items = Vec::new();
		let mut joins = Vec::new();
		for view in ordered.into_iter().skip(start) {
			if SessionOpened::opens(view.post) {
				joins.push(view);
			} else if view.post.intent() != PostIntent::Log {
				self.push_note(&mut items, &mut joins);
				items.push(ModelPost::Post(view));
			}
		}
		self.push_note(&mut items, &mut joins);
		items
	}

	/// The authored posts in the order a model reads them: by [`PostId`],
	/// except that a function call is followed at once by its output, so a
	/// post landing between the two, ie a person's note typed mid tool call,
	/// is read after the output and never breaks a provider's pairing of
	/// call and output. A call with no output yet holds nothing back.
	fn model_order(&self) -> Vec<AuthoredPost<'_>> {
		let answered = self
			.posts
			.iter()
			.filter_map(|post| match AgentPost::new(post)? {
				AgentPost::FunctionCallOutput(output) => {
					output.call_id().to_string().xsome()
				}
				_ => None,
			})
			.collect::<HashSet<_>>();
		let mut ordered = Vec::with_capacity(self.posts.len());
		let mut held = Vec::new();
		let mut pending = HashSet::<String>::default();
		for view in self.authored_posts() {
			match AgentPost::new(view.post) {
				Some(AgentPost::FunctionCall(call)) => {
					if answered.contains(call.call_id()) {
						pending.insert(call.call_id().to_string());
					}
					ordered.push(view);
				}
				Some(AgentPost::FunctionCallOutput(output)) => {
					pending.remove(output.call_id());
					ordered.push(view);
					if pending.is_empty() {
						ordered.append(&mut held);
					}
				}
				_ if pending.is_empty() => ordered.push(view),
				_ => held.push(view),
			}
		}
		ordered.append(&mut held);
		ordered
	}

	/// Push the note of `joins` onto `items`, if any joined, leaving none.
	fn push_note<'a>(
		&self,
		items: &mut Vec<ModelPost<'a>>,
		joins: &mut Vec<AuthoredPost<'a>>,
	) {
		if !joins.is_empty() {
			items.push(ModelPost::Note(self.join_note(joins)));
			joins.clear();
		}
	}

	/// Merge stored records under these: the stored posts and metadata,
	/// then every actor of these records over the stored ones, so
	/// the authored definitions win while the stored authors still resolve.
	pub fn merge_stored(&mut self, stored: ThreadRecords) {
		let ThreadRecords {
			actors,
			posts,
			metas,
			changed: _,
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

	/// Take what changed since window sync last took it: each changed post as it
	/// stands, its author and its metadata. Empty when nothing changed.
	pub fn take_changes(&mut self) -> ThreadChanges {
		let mut changes = ThreadChanges::default();
		for id in core::mem::take(&mut self.changed) {
			let Some(post) = self.posts.iter().find(|post| post.id() == id)
			else {
				continue;
			};
			if let Some(actor) = self.actors.get(&post.author())
				&& !changes.actors.iter().any(|known| known.id() == actor.id())
			{
				changes.actors.push(actor.clone());
			}
			if let Some(meta) = self.metas.get(&id) {
				changes.metas.push(meta.clone());
			}
			changes.posts.push(post.clone());
		}
		changes
	}

	/// Whether anything changed since window sync last took it.
	pub fn has_changes(&self) -> bool { !self.changed.is_empty() }

	/// Apply `changes`, another window's, in [`PostId`] order and without
	/// marking them changed again: an actor these records lack is added, the
	/// rest are replaced.
	pub fn apply_changes(&mut self, changes: &ThreadChanges) {
		for actor in &changes.actors {
			self.actors
				.entry(actor.id())
				.or_insert_with(|| actor.clone());
		}
		for post in &changes.posts {
			self.upsert_quiet(post.clone());
		}
		for meta in &changes.metas {
			self.metas.insert(meta.post_id, meta.clone());
		}
		self.posts.sort_by_key(|post| post.id());
	}

	/// Every record here as changes, for a window joining the thread.
	pub fn as_changes(&self) -> ThreadChanges {
		ThreadChanges {
			actors: self.actors.values().cloned().collect(),
			posts: self.posts.clone(),
			metas: self.metas.values().cloned().collect(),
		}
	}

	// ── response metadata ───────────────────────────────────────────────

	/// Set the metadata of its post, for window sync to apply.
	pub fn set_meta(&mut self, meta: ResponseMeta) {
		self.mark_changed(meta.post_id);
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
			if !self.changed.contains(&post.id()) {
				self.changed.push(post.id());
			}
		}
	}
}

/// One window's changes to its records, applied to every other window of
/// the same stored thread ([`ThreadRecords::take_changes`]).
#[derive(Debug, Default, Clone)]
pub struct ThreadChanges {
	/// The authors of [`Self::posts`].
	pub actors: Vec<Actor>,
	/// Each post as it stands, a post in progress included.
	pub posts: Vec<Post>,
	/// The metadata of [`Self::posts`].
	pub metas: Vec<ResponseMeta>,
}

impl ThreadChanges {
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

/// One item of a model's input ([`ThreadRecords::model_posts`]).
#[derive(Debug, Clone)]
pub enum ModelPost<'a> {
	/// A post that is not a log, sent in the role its author has to the
	/// reading agent.
	Post(AuthoredPost<'a>),
	/// The note of a run of joins ([`ThreadRecords::join_note`]), sent as
	/// it is in the user role whoever joined, the reading agent included:
	/// something that happened, never something the agent said.
	Note(String),
}

impl<'a> ModelPost<'a> {
	/// The post this item is, a note being none.
	pub fn post(&self) -> Option<AuthoredPost<'a>> {
		match self {
			Self::Post(post) => Some(*post),
			Self::Note(_) => None,
		}
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
/// `Actor` carrying behavior for an `ActorRef`.
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
/// despawns the entity. An entry continuing its thread despawns it unread
/// ([`Thread::enter`]).
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
	/// its seed is kept ([`Thread::enter`]).
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
			// through a markup `<slot/>` are still found; actors never nest, so
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

/// Locate and mutate the [`ThreadRecords`] of the window a behavior entity
/// belongs to. Behavior entities ([`ActorRef`]) are descendants of their
/// window; the records live on the window.
#[derive(SystemParam)]
pub struct ThreadRecordsQuery<'w, 's> {
	ancestors: Query<'w, 's, &'static ChildOf>,
	actor_refs: Query<'w, 's, &'static ActorRef>,
	thread_records:
		Query<'w, 's, (&'static Thread, &'static mut ThreadRecords)>,
	synced: Query<'w, 's, &'static mut SyncedPosts>,
}

impl ThreadRecordsQuery<'_, '_> {
	/// Walk ancestors to the window carrying the [`ThreadRecords`].
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

	/// The window's records.
	pub fn records(&self, entity: Entity) -> Result<&ThreadRecords> {
		let thread_entity = self.thread_entity(entity)?;
		Ok(self.thread_records.get(thread_entity)?.1)
	}

	/// Mutable access to the window's records.
	pub fn records_mut(
		&mut self,
		entity: Entity,
	) -> Result<Mut<'_, ThreadRecords>> {
		let thread_entity = self.thread_entity(entity)?;
		Ok(self.thread_records.get_mut(thread_entity)?.1)
	}

	/// Append a post to the window's records.
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

	/// A function call is followed at once by its output in what a model
	/// reads: a post and a join landing between them are read after the
	/// output, a call without an output holds nothing back, and a cursor
	/// reads on from its place in that order.
	#[beet_core::test]
	fn model_order_pairs_each_call_with_its_output() {
		let thread = ThreadId::new_now();
		let person = ActorId::new_now();
		let agent = ActorId::new_now();
		let mut records = ThreadRecords::new();
		records.insert_actor(Actor::new_with_id(
			person,
			"Cam",
			ActorKind::User,
		));
		records.insert_actor(Actor::new_with_id(
			agent,
			"Bot",
			ActorKind::Agent,
		));
		let text = |author, text: &str| {
			AgentPost::new_text(author, thread, text, PostStatus::Completed)
		};
		let call = |call_id: &str| {
			AgentPost::new_function_call(
				agent,
				thread,
				"wait",
				call_id,
				"{}",
				PostStatus::Completed,
			)
		};
		let output = |call_id: &str| {
			AgentPost::new_function_call_output(
				agent,
				thread,
				call_id.to_string(),
				"done".to_string(),
				None,
				PostStatus::Completed,
			)
		};
		let first_call = call("first");
		let cursor = first_call.id();
		for post in [
			text(person, "go"),
			first_call,
			text(person, "by the way"),
			Post::new_log(person, thread, &SessionOpened::default()).unwrap(),
			output("first"),
			call("unanswered"),
			text(person, "still there?"),
		] {
			records.upsert_post(post);
		}
		let read = |after| {
			records
				.model_posts(after)
				.into_iter()
				.map(|item| match item {
					ModelPost::Note(_) => "note".to_string(),
					ModelPost::Post(view) => match AgentPost::new(view.post) {
						Some(AgentPost::FunctionCall(_)) => "call".into(),
						Some(AgentPost::FunctionCallOutput(_)) => {
							"output".into()
						}
						_ => view.post.to_string(),
					},
				})
				.collect::<Vec<_>>()
		};
		read(None).xpect_eq(
			[
				"go",
				"call",
				"output",
				"by the way",
				"note",
				"call",
				"still there?",
			]
			.map(String::from)
			.to_vec(),
		);
		read(Some(cursor)).xpect_eq(
			["output", "by the way", "note", "call", "still there?"]
				.map(String::from)
				.to_vec(),
		);
	}

	/// A post's author is the named actor, else the first User actor by id,
	/// and with neither there is none.
	#[beet_core::test]
	fn author_falls_back_to_the_first_user() {
		let first = ActorId::from_u128(1);
		let second = ActorId::from_u128(2);
		let agent = ActorId::from_u128(3);
		let mut records = ThreadRecords::new();
		records.insert_actor(Actor::new_with_id(
			agent,
			"Bot",
			ActorKind::Agent,
		));
		records
			.author(None)
			.unwrap_err()
			.to_string()
			.xpect_contains("holds no User actor");
		records.insert_actor(Actor::new_with_id(second, "B", ActorKind::User));
		records.insert_actor(Actor::new_with_id(first, "A", ActorKind::User));
		records.author(None).unwrap().xpect_eq(first);
		records.author(Some(agent)).unwrap().xpect_eq(agent);
		records.author(Some(ActorId::from_u128(4))).unwrap_err();
	}
}
