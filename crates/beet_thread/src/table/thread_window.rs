use crate::prelude::*;
#[cfg(feature = "action")]
use beet_action::prelude::*;
use beet_core::prelude::*;

/// A materialized view of a [`Thread`], the runtime truth-for-the-scene.
///
/// One source of truth (the database) projected twice: the window is the
/// resident hot slice the scene streams into, renders, and sends as model
/// context (it is, literally, the model's context window). It is **always** a
/// projection, never a source of truth.
///
/// Author resolution is in memory (`window.actor(post.author())`), with no
/// entity walk; ordering is by [`PostId`] (a time-sortable [`Uuid7`]).
#[derive(Debug, Default, Clone, Component)]
pub struct ThreadWindow {
	/// All actors that appear in the posts of this window, keyed by [`ActorId`].
	actors: HashMap<ActorId, Actor>,
	/// Ordered list of posts (by [`PostId`]).
	posts: Vec<Post>,
	/// Per-post response metadata, used for `previous_response_id` chaining.
	metas: HashMap<PostId, ResponseMeta>,
}

impl ThreadWindow {
	pub fn new() -> Self { Self::default() }

	// ── actors ──────────────────────────────────────────────────────────

	pub fn actors(&self) -> &HashMap<ActorId, Actor> { &self.actors }

	/// Resolve an actor by id, in memory.
	#[track_caller]
	pub fn actor(&self, id: ActorId) -> Result<&Actor> {
		self.actors
			.get(&id)
			.ok_or_else(|| bevyhow!("No actor {id} in window"))
	}

	/// Insert (or replace) an actor in the window.
	pub fn insert_actor(&mut self, actor: Actor) -> ActorId {
		let id = actor.id();
		self.actors.insert(id, actor);
		id
	}

	// ── posts ───────────────────────────────────────────────────────────

	pub fn posts(&self) -> &[Post] { &self.posts }
	pub fn is_empty(&self) -> bool { self.posts.is_empty() }
	pub fn last_post(&self) -> Option<&Post> { self.posts.last() }

	/// Append a post, or replace the existing post with the same id. New posts
	/// keep insertion (ie [`PostId`]) order; modified posts update in place.
	pub fn upsert_post(&mut self, post: Post) {
		match self
			.posts
			.iter_mut()
			.find(|existing| existing.id() == post.id())
		{
			Some(existing) => *existing = post,
			None => self.posts.push(post),
		}
	}

	/// Join each post with its authoring [`Actor`] from the window.
	pub fn post_views(&self) -> impl Iterator<Item = PostView<'_>> {
		self.posts.iter().filter_map(|post| {
			self.actors
				.get(&post.author())
				.map(|actor| PostView { post, actor })
		})
	}

	/// Post views after an exclusive cursor. A missing or unknown cursor
	/// returns every view (the caller has seen none of these posts).
	pub fn posts_after(&self, after: Option<PostId>) -> Vec<PostView<'_>> {
		let views = self.post_views().collect::<Vec<_>>();
		let Some(after) = after else { return views };
		match views.iter().position(|view| view.post.id() == after) {
			Some(i) => views[i + 1..].to_vec(),
			None => views,
		}
	}

	// ── response metadata ───────────────────────────────────────────────

	pub fn set_meta(&mut self, meta: ResponseMeta) {
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

	/// Adopt a stored thread's records, replacing the authored seed with the
	/// persisted conversation. Used by seed-hash load (see `load_thread`).
	pub fn load_records(&mut self, actors: Vec<Actor>, posts: Vec<Post>) {
		actors.into_iter().for_each(|actor| {
			self.insert_actor(actor);
		});
		self.posts = posts;
		self.posts.sort_by_key(|post| post.id());
	}

	/// Trim the window for an endless loop, keeping the request bounded.
	///
	/// Retains every system/developer post (the seed prompt), the last
	/// `keep_posts` other posts, and within those the last `keep_media` media
	/// (image/video/audio) posts, dropping older ones. Any function-call-output
	/// post whose paired call was dropped is dropped too, so providers never see
	/// an orphaned tool output. Call between turns, when every call is paired.
	pub fn prune(&mut self, keep_posts: usize, keep_media: usize) {
		// the index the retained non-system tail begins at
		let tail_start = self
			.posts
			.iter()
			.filter(|post| !self.is_seed_kind(post))
			.count()
			.saturating_sub(keep_posts);
		let mut media_seen = self
			.posts
			.iter()
			.filter(|post| post.media_type().is_media())
			.count();
		let mut non_seed_seen = 0;
		let mut retained_calls = HashSet::<String>::default();
		let mut posts = std::mem::take(&mut self.posts);
		posts.retain(|post| {
			let is_media = post.media_type().is_media();
			// count from the front so `media_seen`/`non_seed_seen` mark position
			let media_index = media_seen - if is_media { 1 } else { 0 };
			if is_media {
				media_seen -= 1;
			}
			if self.is_seed_kind(post) {
				return true;
			}
			let index = non_seed_seen;
			non_seed_seen += 1;
			if index < tail_start {
				return false;
			}
			// within the tail: older media beyond the last `keep_media` drop out
			if is_media && media_index >= keep_media {
				return false;
			}
			// tool outputs only survive alongside their call
			let agent_post = post.as_agent_post();
			match &agent_post {
				AgentPost::FunctionCall(call) => {
					retained_calls.insert(call.call_id().to_string());
					true
				}
				AgentPost::FunctionCallOutput(output) => {
					retained_calls.contains(output.call_id())
				}
				_ => true,
			}
		});
		self.posts = posts;
		// drop metas for pruned posts so `stored_response` never chains to them
		let retained = &self.posts;
		self.metas
			.retain(|post_id, _| retained.iter().any(|post| post.id() == *post_id));
	}

	/// Whether a post is part of the immutable seed (system/developer authored).
	fn is_seed_kind(&self, post: &Post) -> bool {
		self.actors
			.get(&post.author())
			.map(|actor| {
				matches!(actor.kind(), ActorKind::System | ActorKind::Developer)
			})
			.unwrap_or(false)
	}
}

// ═══════════════════════════════════════════════════════════════════════
// PostView
// ═══════════════════════════════════════════════════════════════════════

/// A post joined with its authoring [`Actor`], sourced from a [`ThreadWindow`].
#[derive(Debug, Clone, Copy)]
pub struct PostView<'a> {
	pub post: &'a Post,
	pub actor: &'a Actor,
}

impl std::ops::Deref for PostView<'_> {
	type Target = Post;
	fn deref(&self) -> &Self::Target { self.post }
}

impl PostView<'_> {
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
/// definition lives in the [`ThreadWindow`]; the behavior scene carries only
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
/// ([`ThreadWindow::reduce`]): it resolves the seed's author (its actor parent)
/// and thread, hoists the materialized [`Post`] into the [`ThreadWindow`], and
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
#[derive(Clone, Component)]
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
// The reduction: author scene -> ThreadWindow + behavior scene
// ═══════════════════════════════════════════════════════════════════════

impl ThreadWindow {
	/// Reduce every freshly-spawned authored [`Thread`] into a [`ThreadWindow`] plus
	/// a lean behavior scene.
	///
	/// For each thread lacking a window, walk its descendant actors in author order
	/// and: hoist each [`Actor`] into `window.actors`, hoist each [`SeedPost`] into
	/// `window.posts`, then rewrite the actor entity. An actor that carries behavior
	/// (an [`Action`]) becomes an [`ActorRef`] keeping its behavior and tools; a
	/// seed-only actor (eg the system prompt) is despawned, leaving no turn behind.
	///
	/// Idempotent: gated on `Without<ThreadWindow>`, so it runs once per thread.
	pub fn reduce(
		mut commands: Commands,
		threads: Query<(Entity, &Thread), Without<ThreadWindow>>,
		children: Query<&Children>,
		actors: Query<&Actor>,
		seeds: Query<&SeedPost>,
		#[cfg(feature = "action")] behaviors: Query<(), With<ActionMeta>>,
	) -> Result {
		for (thread_entity, thread) in threads.iter() {
			let thread_id = thread.id();
			let mut window = ThreadWindow::new();

			// descendant actor entities, in author (DFS) order
			let actor_entities = children
				.iter_descendants(thread_entity)
				.filter(|entity| actors.contains(*entity))
				.collect::<Vec<_>>();

			for actor_entity in actor_entities {
				let actor = actors.get(actor_entity)?.clone();
				let actor_id = window.insert_actor(actor);

				// hoist this actor's seed posts, despawning their entities. Scan all
				// descendants (not just direct children) so seeds authored through a
				// markup `<Slot/>` are still found; actors never nest, so every
				// `SeedPost` under an actor entity is unambiguously its own.
				children
					.iter_descendants(actor_entity)
					.filter_map(|child| {
						seeds.get(child).ok().map(|seed| (child, seed))
					})
					.for_each(|(child, seed)| {
						window.upsert_post(
							seed.clone().into_post(actor_id, thread_id),
						);
						commands.entity(child).despawn();
					});

				// rewrite the actor: behavior -> ActorRef, seed-only -> despawn
				#[cfg(feature = "action")]
				let has_behavior = behaviors.contains(actor_entity);
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

			window.posts.sort_by_key(|post| post.id());
			commands.entity(thread_entity).insert(window);
		}
		Ok(())
	}

	/// Run [`ThreadWindow::reduce`] immediately and flush, for manual consumers that
	/// pump the world directly (eg `run_oneshot`, tests) rather than the schedule.
	pub fn reduce_now(world: &mut World) {
		let _ = world.run_system_cached::<Result, _, _>(Self::reduce);
		world.flush();
	}

	/// Spawn an authored thread `scene` and reduce it into a [`ThreadWindow`] plus
	/// behavior scene in one step, returning the root entity. The behavior is *not*
	/// triggered; the caller drives it (eg `entity.call(())`), so the window is
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

	/// Compute the seed hash of a freshly-reduced [`ThreadWindow`]: the authored
	/// actor definitions and seed-post content/author, excluding volatile post ids
	/// and timestamps. Editing the seed changes the hash (forks a thread); editing
	/// only behavior does not.
	#[cfg(feature = "action")]
	pub fn seed_hash(&self) -> u64 {
		use std::hash::Hash;
		use std::hash::Hasher;
		let mut hasher = std::collections::hash_map::DefaultHasher::new();

		let mut actors = self.actors().values().collect::<Vec<_>>();
		actors.sort_by_key(|actor| actor.id());
		for actor in actors {
			actor.id().hash(&mut hasher);
			actor.name().hash(&mut hasher);
			actor.kind().hash(&mut hasher);
		}

		let mut posts = self.posts().iter().collect::<Vec<_>>();
		posts.sort_by_key(|post| post.id());
		for post in posts {
			post.author().hash(&mut hasher);
			post.intent().hash(&mut hasher);
			post.body_bytes().hash(&mut hasher);
		}

		hasher.finish()
	}
}

// ═══════════════════════════════════════════════════════════════════════
// ThreadWindowQuery: mutate the thread window from a behavior entity
// ═══════════════════════════════════════════════════════════════════════

/// Locate and mutate the [`ThreadWindow`] of the thread a behavior entity
/// belongs to. Behavior entities ([`ActorRef`]) are descendants of their
/// thread; the window lives on the thread.
#[derive(SystemParam)]
pub struct ThreadWindowQuery<'w, 's> {
	ancestors: Query<'w, 's, &'static ChildOf>,
	actor_refs: Query<'w, 's, &'static ActorRef>,
	windows: Query<'w, 's, (&'static Thread, &'static mut ThreadWindow)>,
}

impl ThreadWindowQuery<'_, '_> {
	/// Walk ancestors to the thread entity carrying the [`ThreadWindow`].
	pub fn thread_entity(&self, entity: Entity) -> Result<Entity> {
		self.ancestors
			.iter_ancestors_inclusive(entity)
			.find(|ancestor| self.windows.contains(*ancestor))
			.ok_or_else(|| bevyhow!("No ThreadWindow in ancestors of {entity}"))
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
		Ok(self.windows.get(thread_entity)?.0.id())
	}

	/// Mutable access to the thread's window.
	pub fn window_mut(
		&mut self,
		entity: Entity,
	) -> Result<Mut<'_, ThreadWindow>> {
		let thread_entity = self.thread_entity(entity)?;
		Ok(self.windows.get_mut(thread_entity)?.1)
	}

	/// Append a post to the thread's window.
	pub fn push_post(&mut self, entity: Entity, post: Post) -> Result {
		self.window_mut(entity)?.upsert_post(post);
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The reduction hoists every actor and seed post into the window, despawns
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
		ThreadWindow::reduce_now(&mut world);

		// window holds both actors and the one seed post, author resolvable
		let window = world.get::<ThreadWindow>(thread).unwrap();
		window.actors().len().xpect_eq(2);
		window.posts().len().xpect_eq(1);
		window.posts()[0]
			.to_string()
			.xpect_eq("sys prompt".to_string());
		let author = window.posts()[0].author();
		window
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

	/// Pruning keeps the seed prompt, the last `keep_posts` other posts and the
	/// last `keep_media` media posts, and never orphans a tool output.
	#[beet_core::test]
	fn prune_bounds_the_window() {
		let thread = ThreadId::new_now();
		let system = ActorId::new_now();
		let user = ActorId::new_now();
		let agent = ActorId::new_now();
		let mut window = ThreadWindow::new();
		window.insert_actor(Actor::new_with_id(system, "Sys", ActorKind::System));
		window.insert_actor(Actor::new_with_id(user, "Cam", ActorKind::User));
		window.insert_actor(Actor::new_with_id(agent, "Bot", ActorKind::Agent));
		window.upsert_post(AgentPost::new_text(
			system,
			thread,
			"seed prompt",
			PostStatus::Completed,
		));
		// three cycles of photo -> call -> output
		for cycle in 0..3 {
			window.upsert_post(AgentPost::new_bytes(
				user,
				thread,
				MediaType::Jpeg,
				vec![cycle as u8],
				None,
				PostStatus::Completed,
			));
			window.upsert_post(AgentPost::new_function_call(
				agent,
				thread,
				"act",
				format!("call-{cycle}"),
				"{}",
				PostStatus::Completed,
			));
			window.upsert_post(AgentPost::new_function_call_output(
				agent,
				thread,
				format!("call-{cycle}"),
				"done",
				Some("act".into()),
				PostStatus::Completed,
			));
		}
		window.posts().len().xpect_eq(10);

		// keep the last two cycles' posts but only the newest photo
		window.prune(6, 1);
		// seed + (photo dropped, call+output) + (photo, call, output) = 6
		window.posts().len().xpect_eq(6);
		window.posts()[0]
			.to_string()
			.xpect_eq("seed prompt".to_string());
		window
			.posts()
			.iter()
			.filter(|post| post.media_type().is_media())
			.count()
			.xpect_eq(1);
		// every retained output still has its call
		let calls = window
			.posts()
			.iter()
			.filter_map(|post| match post.as_agent_post() {
				AgentPost::FunctionCall(call) => {
					Some(call.call_id().to_string())
				}
				_ => None,
			})
			.collect::<Vec<_>>();
		window
			.posts()
			.iter()
			.filter_map(|post| match post.as_agent_post() {
				AgentPost::FunctionCallOutput(output) => {
					Some(output.call_id().to_string())
				}
				_ => None,
			})
			.all(|call_id| calls.contains(&call_id))
			.xpect_true();

		// a tighter prune drops whole cycles, never leaving a leading orphan
		window.prune(2, 1);
		window.posts()[0]
			.to_string()
			.xpect_eq("seed prompt".to_string());
		let first_non_seed = window.posts()[1].as_agent_post();
		matches!(first_non_seed, AgentPost::FunctionCallOutput(_))
			.xpect_false();
	}

	/// Streaming-shaped window mutation: upsert appends new posts and replaces
	/// existing ones in place, ordered by id.
	#[beet_core::test]
	fn upsert_appends_then_replaces() {
		let thread = ThreadId::new_now();
		let author = ActorId::new_now();
		let mut window = ThreadWindow::new();
		window.insert_actor(Actor::new_with_id(author, "A", ActorKind::Agent));

		let mut post =
			AgentPost::new_text(author, thread, "hel", PostStatus::InProgress);
		window.upsert_post(post.clone());
		window.posts().len().xpect_eq(1);

		// same id streamed further: replaced in place, not appended
		post.set_text("hello");
		post.set_status(PostStatus::Completed);
		window.upsert_post(post);
		window.posts().len().xpect_eq(1);
		window
			.last_post()
			.unwrap()
			.to_string()
			.xpect_eq("hello".to_string());
		window.last_post().unwrap().in_progress().xpect_false();
	}
}
