//! Entering a thread: which stored thread a window opens onto, created or
//! continued, and the records it enters with.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Declare where a thread's records persist, in the store at or above the
/// thread: `<Thread {(Sequence, MountThreadStore{path:"threads/coach/{claim}"})}>`.
///
/// The path names the thread. Each `{param}` is filled from the request
/// entering it ([`ThreadEntry::fill_path`]), and the newest thread in the
/// filled directory continues, so `threads/coach/{claim}` keeps one thread
/// per claim. A deferred mount: [`Thread::enter`] fills it, picks the stored
/// thread and only then inserts the [`ThreadStore`], so the persistence sync
/// never flushes a thread before it knows which one it is.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct MountThreadStore {
	/// The directory the thread's records persist under, each `{param}` the
	/// entering request's value for it.
	pub path: RelPath,
	/// Persist and never continue: every entry creates a thread, as if it
	/// carried `--new`.
	pub new: bool,
}

impl MountThreadStore {
	/// The [`ThreadStore`] `entry` opens: the filled path in `store`.
	///
	/// ## Errors
	///
	/// Errors when the path names a param the request lacks or one whose
	/// value would leave the mount's prefix.
	pub fn build(
		&self,
		store: &BlobStore,
		entry: &ThreadEntry,
	) -> Result<ThreadStore> {
		entry
			.fill_path(&self.path)?
			.xmap(|path| store.with_subdir(path))
			.xmap(BlobThreadStore::new)
			.xmap(ThreadStore::new)
			.xok()
	}
}

/// The params an entry reads off its request, taken by [`ThreadRoute`] and
/// [`RunThread`], so `--help` lists them and a strict server validates them.
#[derive(Debug, Default, Clone, Reflect)]
pub struct ThreadEntryParams {
	/// Create a thread rather than continuing the newest in the mount's
	/// directory.
	pub new: bool,
	/// Continue this stored thread rather than the newest.
	pub thread: Option<ThreadId>,
	/// The post whose action asked for this entry, `<thread id>/<post id>`:
	/// a thread the entry creates records it as [`Thread::spawned_by`], and
	/// the entry's join as its cause.
	pub spawned_by: Option<PostRef>,
	/// Create a thread continuing the history of this post,
	/// `<thread id>/<post id>`, which the mount's store must hold
	/// ([`Thread::branch_of`]).
	pub branch_of: Option<PostRef>,
}

/// One request entering a thread: the request's parts, which fill the
/// mount's path, its [`ThreadEntryParams`], and the message it posts, if
/// any.
#[derive(Debug, Default, Clone)]
pub struct ThreadEntry {
	/// The parts of the request entering.
	parts: RequestParts,
	/// The params of the request entering.
	params: ThreadEntryParams,
	/// The request's message, filled, posted once the thread is entered.
	message: Option<String>,
	/// The author of the message, else the thread's first User actor by id.
	actor: Option<ActorId>,
}

impl ThreadEntry {
	/// An entry for the request `parts`, reading `params`.
	pub fn new(parts: RequestParts, params: ThreadEntryParams) -> Self {
		Self {
			parts,
			params,
			message: None,
			actor: None,
		}
	}

	/// An entry for the request `parts`, its params parsed off them.
	pub fn parse(parts: RequestParts) -> Result<Self> {
		let params = parts.parse_params::<ThreadEntryParams>()?;
		Self::new(parts, params).xok()
	}

	/// The params of the request entering.
	pub fn params(&self) -> &ThreadEntryParams { &self.params }

	/// Post `message` once the thread is entered, each `{param}` filled from
	/// the request, authored by `actor`, else the thread's first User actor
	/// by id.
	///
	/// ## Errors
	///
	/// Errors when `message` names a param the request lacks, or opens a `{`
	/// it never closes.
	pub fn with_message(
		mut self,
		message: &str,
		actor: Option<ActorId>,
	) -> Result<Self> {
		self.message = Some(self.fill(message)?);
		self.actor = actor;
		self.xok()
	}

	/// `text` with each `{param}` replaced by the request's value for it, the
	/// values of a param holding several, ie a greedy path param's segments,
	/// joined by `/`. `{{` and `}}` spell a literal brace.
	///
	/// ## Errors
	///
	/// Errors on a param the request lacks and on an unclosed `{`.
	pub fn fill(&self, text: &str) -> Result<String> {
		Self::fill_with(text, &self.parts, |_, value| value.xok())
	}

	/// `path` filled as [`Self::fill`] fills text, refusing a value holding a
	/// `..` segment, starting with `/` or empty, naming the param and the
	/// mount. [`RelPath`] would drop such a value silently, so the check is
	/// on the value before it is joined: what it protects is the mount's own
	/// prefix, never just the store.
	///
	/// ## Errors
	///
	/// Errors as [`Self::fill`], and on an escaping value.
	pub fn fill_path(&self, path: &RelPath) -> Result<RelPath> {
		Self::fill_with(path.as_str(), &self.parts, |name, value| {
			let refusal = if value.is_empty() {
				"is empty"
			} else if value.starts_with('/') {
				"starts with `/`"
			} else if value.split('/').any(|segment| segment == "..") {
				"holds a `..` segment"
			} else {
				return value.xok();
			};
			bevybail!(
				"the param `{name}` filling the mount `{path}` {refusal}: `{value}`"
			)
		})?
		.xmap(RelPath::new)
		.xok()
	}

	/// The fill shared by text and paths, `check` answering each param's
	/// joined value as it lands, or refusing it.
	fn fill_with(
		text: &str,
		parts: &RequestParts,
		check: impl Fn(&str, String) -> Result<String>,
	) -> Result<String> {
		let mut out = String::with_capacity(text.len());
		let mut chars = text.chars().peekable();
		while let Some(next) = chars.next() {
			match (next, chars.peek()) {
				('{', Some('{')) | ('}', Some('}')) => {
					out.push(next);
					chars.next();
				}
				('{', _) => {
					let mut name = String::new();
					loop {
						match chars.next() {
							Some('}') => break,
							Some(ch) => name.push(ch),
							None => {
								bevybail!(
									"`{text}` opens a `{{` it never closes"
								)
							}
						}
					}
					let name = name.trim();
					// a greedy path param holds a value per segment, ie
					// `build/*rubric`'s `acme/01-plan`, so they join as the
					// path they were
					let value = parts
						.get_params(name)
						.filter(|values| !values.is_empty())
						.ok_or_else(|| {
							bevyhow!(
								"`{text}` names the param `{name}`, which the request `{}` lacks",
								parts.path_string()
							)
						})?
						.join("/");
					out.push_str(&check(name, value)?);
				}
				(other, _) => out.push(other),
			}
		}
		out.xok()
	}

	/// The stored thread this entry continues, `None` when it creates one.
	///
	/// ## Errors
	///
	/// Errors when `--thread` names a thread the store lacks, when it is
	/// given beside `--branch-of`, or when `--branch-of` names a post the
	/// store lacks.
	#[cfg(feature = "action")]
	async fn find(
		&self,
		mount: &MountThreadStore,
		store: &ThreadStore,
	) -> Result<Option<Thread>> {
		match (&self.params.thread, &self.params.branch_of) {
			(Some(_), Some(_)) => bevybail!(
				"`--branch-of` creates a thread, so it cannot continue `--thread`"
			),
			(Some(id), None) => store
				.thread(*id)
				.await?
				.ok_or_else(|| {
					bevyhow!("the mount `{}` holds no thread {id}", mount.path)
				})?
				.xsome()
				.xok(),
			(None, Some(point)) => {
				Self::assert_branch_point(mount, store, point).await?;
				Ok(None)
			}
			(None, None) if self.params.new || mount.new => Ok(None),
			(None, None) => store
				.threads()
				.await?
				.into_iter()
				.max_by_key(|thread| (thread.created(), thread.id()))
				.xok(),
		}
	}

	/// Refuse a branch point the mount's store lacks: a branch lives in its
	/// parent's store.
	#[cfg(feature = "action")]
	async fn assert_branch_point(
		mount: &MountThreadStore,
		store: &ThreadStore,
		point: &PostRef,
	) -> Result {
		let parent = store.thread(point.thread()).await?.ok_or_else(|| {
			bevyhow!(
				"`--branch-of={point}` names a thread the mount `{}` lacks",
				mount.path
			)
		})?;
		match store
			.history(&parent)
			.await?
			.iter()
			.any(|post| post.id() == point.post())
		{
			true => Ok(()),
			false => bevybail!(
				"`--branch-of={point}` names a post the thread {} lacks",
				parent.id()
			),
		}
	}

	/// Refuse params only a store can honour, for a thread mounting none.
	#[cfg(feature = "action")]
	fn assert_storeless(&self) -> Result {
		if let Some(id) = self.params.thread {
			bevybail!(
				"`--thread={id}` names a stored thread, but the thread mounts no store"
			);
		}
		if let Some(point) = &self.params.branch_of {
			bevybail!(
				"`--branch-of={point}` branches in its parent's store, but the thread mounts no store"
			);
		}
		Ok(())
	}

	/// The record of a thread this entry creates: `fresh`, lineage recorded.
	#[cfg(feature = "action")]
	fn create(&self, mut fresh: Thread) -> Thread {
		fresh.set_spawned_by(self.params.spawned_by.clone());
		fresh.set_branch_of(self.params.branch_of.clone());
		fresh
	}

	/// The join this entry writes.
	#[cfg(feature = "action")]
	fn opened(&self) -> SessionOpened {
		SessionOpened {
			spawned_by: self.params.spawned_by.clone(),
		}
	}

	/// Post the request's message into `records`, if the entry carries one.
	///
	/// ## Errors
	///
	/// Errors when no actor is named and the records hold no User actor.
	#[cfg(feature = "action")]
	fn post_message(
		&self,
		records: &mut ThreadRecords,
		thread: ThreadId,
	) -> Result {
		let Some(message) = &self.message else {
			return Ok(());
		};
		let author = match self.actor {
			Some(actor) => records.actor(actor)?.id(),
			None => records
				.actors()
				.values()
				.filter(|actor| actor.kind() == ActorKind::User)
				.map(Actor::id)
				.min()
				.ok_or_else(|| {
					bevyhow!(
						"the request's message `{message}` names no actor, and the thread holds no User actor to post it"
					)
				})?,
		};
		records.upsert_post(AgentPost::new_text(
			author,
			thread,
			message.clone(),
			PostStatus::Completed,
		));
		Ok(())
	}
}

impl Thread {
	/// Enter the thread at `entity` before its behavior runs, the one place
	/// that knows whether its thread is created or continued:
	///
	/// 1. fill its [`MountThreadStore`] from the request, refusing an
	///    escaping value;
	/// 2. find the stored thread `entry` continues ([`ThreadEntryParams`],
	///    [`MountThreadStore::new`], else the newest in the directory);
	/// 3. creating: record `spawned_by` and `branch_of`, and for a fresh
	///    thread resolve each [`SeedSource`] in author order and reduce the
	///    scene keeping its seed posts;
	/// 4. continuing, or branching: reduce the scene dropping its seed posts,
	///    reading no seed source;
	/// 5. load the stored records, post the entry's join, then the request's
	///    message if the entry carries one.
	///
	/// A thread mounting no store persists nothing, so every entry creates
	/// it. A branch enters with its parent's history up to the branch point,
	/// so it opens with its parent's posts and reads no seed. Once per entity:
	/// the mount is consumed by the first entry.
	///
	/// ## Errors
	///
	/// Errors when the mount's path cannot be filled, when the mount has no
	/// store above the thread, when the params name a thread or post the
	/// store lacks, when a seed cannot be read, naming it, and when the
	/// request's message has no author.
	#[cfg(feature = "action")]
	pub async fn enter(
		world: AsyncWorld,
		entity: Entity,
		entry: ThreadEntry,
	) -> Result {
		// 1. fill the mount. It stays on the thread until the reduction
		// consumes it, so no scheduled reduce runs ahead of this entry.
		let mount_entry = entry.clone();
		let mounted = world
			.with(move |world: &mut World| -> Result<Option<_>> {
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
				let thread_store = mount.build(&store, &mount_entry)?;
				(mount, thread_store).xsome().xok()
			})
			.await?;

		// 2. find the stored thread
		let stored = match &mounted {
			Some((mount, store)) => {
				store.store_try_create().await?;
				entry.find(mount, store).await?
			}
			None => {
				entry.assert_storeless()?;
				None
			}
		};
		let creating = stored.is_none();
		// a branch opens with its parent's posts, so only a fresh thread seeds
		let keep_seeds = creating && entry.params.branch_of.is_none();

		// 3. creating: resolve each seed source; continuing reads none
		if keep_seeds {
			SeedSource::resolve_all(&world, entity, &entry).await?;
		}
		let creator = entry.clone();
		let record = world
			.with(move |world: &mut World| -> Result<Thread> {
				ThreadRecords::reduce_entered(world, entity, keep_seeds)?;
				let fresh = world
					.get::<Thread>(entity)
					.cloned()
					.ok_or_else(|| bevyhow!("entered {entity} is no thread"))?;
				stored.unwrap_or_else(|| creator.create(fresh)).xok()
			})
			.await?;

		// 5. load the stored records, join, then post the request's message
		let loaded = match &mounted {
			Some((_, store)) => store.load_records(&record).await?,
			None => ThreadRecords::new(),
		};
		world
			.with(move |world: &mut World| -> Result {
				let synced = loaded.posts().iter().map(Post::id).collect();
				let mut records = world
					.get_mut::<ThreadRecords>(entity)
					.ok_or_else(|| bevyhow!("thread {entity} despawned"))?;
				records.merge_stored(loaded);
				records.open_session(record.id(), entry.opened())?;
				entry.post_message(&mut records, record.id())?;
				let mut entity = world.entity_mut(entity);
				entity.insert((record, SyncedPosts::new(synced)));
				if let Some((_, store)) = mounted {
					entity.insert(store);
				}
				Ok(())
			})
			.await
	}
}

#[cfg(all(test, feature = "action"))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// Spawn a thread under a root holding `store`, mounting `mount`, its
	/// System actor seeded with `seed` and a User actor beside it, then enter
	/// it with `request` and flush: the thread's record and records, or the
	/// entry's error.
	async fn enter(
		store: &BlobStore,
		mount: MountThreadStore,
		seed: impl Bundle,
		request: Request,
	) -> Result<(Thread, ThreadRecords)> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.install_type::<ThreadPlugin>();
		let thread = app
			.world_mut()
			.spawn((store.clone(), children![(
				Thread::default(),
				mount,
				children![
					(
						Actor::new_with_id(
							ActorId::from_u128(1),
							"System",
							ActorKind::System,
						),
						children![seed],
					),
					Actor::new_with_id(
						ActorId::from_u128(2),
						"Person",
						ActorKind::User,
					),
				]
			)]))
			.get::<Children>()
			.unwrap()[0];
		let entry = ThreadEntry::parse(request.parts().clone())?;
		app.world_mut()
			.run_async_local_then(async move |world: AsyncWorld| -> Result {
				Thread::enter(world.clone(), thread, entry).await?;
				ThreadStore::flush(&world.entity(thread)).await
			})
			.await?;
		(
			app.world().get::<Thread>(thread).unwrap().clone(),
			app.world().get::<ThreadRecords>(thread).unwrap().clone(),
		)
			.xok()
	}

	/// A mount at `path`.
	fn mount(path: &str) -> MountThreadStore {
		MountThreadStore {
			path: RelPath::new(path),
			new: false,
		}
	}

	/// A request with `params`.
	fn request(params: &[(&str, &str)]) -> Request {
		params
			.iter()
			.fold(Request::get("chat"), |request, (key, value)| {
				request.with_param(key, value)
			})
	}

	/// The text of each message post of `records`, the joins aside.
	fn texts(records: &ThreadRecords) -> Vec<String> {
		records
			.model_posts(None)
			.into_iter()
			.map(|view| view.post.to_string())
			.collect()
	}

	/// The threads stored under `path` in `store`.
	async fn stored(store: &BlobStore, path: &str) -> Vec<Thread> {
		BlobThreadStore::new(store.with_subdir(RelPath::new(path)))
			.threads()
			.await
			.unwrap()
	}

	/// A continued thread reads no seed file and calls no seed route: both
	/// sources would fail if read, and it enters with its stored seed.
	#[beet_core::test]
	async fn a_continued_thread_reads_no_seed() {
		let store = BlobStore::temp();
		store
			.insert(&RelPath::new("seeds/system.md"), "be brief".to_string())
			.await
			.unwrap();
		let (first, _) = enter(
			&store,
			mount("threads"),
			SeedSource::blob(RelPath::new("seeds/system.md")),
			request(&[]),
		)
		.await
		.unwrap();
		let (thread, records) = enter(
			&store,
			mount("threads"),
			(
				SeedSource::blob(RelPath::new("seeds/missing.md")),
				children![SeedSource::route("nowhere")],
			),
			request(&[]),
		)
		.await
		.unwrap();
		thread.id().xpect_eq(first.id());
		texts(&records).xpect_eq(vec!["be brief".to_string()]);
	}

	/// A thread being created reads its seed, and one that cannot be read
	/// fails the entry, naming it.
	#[beet_core::test]
	async fn a_created_thread_reads_its_seed() {
		let store = BlobStore::temp();
		enter(
			&store,
			mount("threads"),
			SeedSource::blob(RelPath::new("seeds/missing.md")),
			request(&[]),
		)
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("<CreatePost src=\"seeds/missing.md\">");
	}

	/// The filled path is the key: two values keep two threads, the same
	/// value continues one.
	#[beet_core::test]
	async fn the_path_names_the_thread() {
		let store = BlobStore::temp();
		let mut ids = Vec::new();
		for claim in ["one", "two", "one"] {
			let (thread, _) = enter(
				&store,
				mount("threads/{claim}"),
				Post::spawn("hi"),
				request(&[("claim", claim)]),
			)
			.await
			.unwrap();
			ids.push(thread.id());
		}
		ids[0].xpect_eq(ids[2]);
		(ids[0] != ids[1]).xpect_true();
		stored(&store, "threads/one").await.len().xpect_eq(1);
		stored(&store, "threads/two").await.len().xpect_eq(1);
	}

	/// A greedy param fills as the path it captured, so the filled path nests.
	#[beet_core::test]
	async fn a_greedy_param_nests() {
		let store = BlobStore::temp();
		let mut request = Request::get("build/acme/01-plan");
		request
			.params_mut()
			.insert_vec("rubric".into(), vec!["acme".into(), "01-plan".into()]);
		enter(
			&store,
			mount("threads/{rubric}"),
			Post::spawn("hi"),
			request,
		)
		.await
		.unwrap();
		stored(&store, "threads/acme/01-plan")
			.await
			.len()
			.xpect_eq(1);
	}

	/// A value escaping the mount's prefix is refused, naming the param.
	#[beet_core::test]
	async fn an_escaping_param_is_refused() {
		for (value, refusal) in [
			("..", "holds a `..` segment"),
			("a/../../b", "holds a `..` segment"),
			("/etc", "starts with `/`"),
			("", "is empty"),
		] {
			enter(
				&BlobStore::temp(),
				mount("threads/{claim}"),
				Post::spawn("hi"),
				request(&[("claim", value)]),
			)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains(format!(
				"the param `claim` filling the mount `threads/{{claim}}` {refusal}"
			));
		}
	}

	/// A `new` mount persists every thread and continues none.
	#[beet_core::test]
	async fn a_new_mount_never_continues() {
		let store = BlobStore::temp();
		for _ in 0..2 {
			enter(
				&store,
				MountThreadStore {
					path: RelPath::new("threads"),
					new: true,
				},
				Post::spawn("hi"),
				request(&[]),
			)
			.await
			.unwrap();
		}
		stored(&store, "threads").await.len().xpect_eq(2);
	}

	/// `--new` creates a thread, which the next entry continues, and
	/// `--thread` continues a stored thread by id.
	#[beet_core::test]
	async fn new_and_thread_pick_the_thread() {
		let store = BlobStore::temp();
		let enter = async |params: &[(&str, &str)]| {
			enter(&store, mount("threads"), Post::spawn("hi"), request(params))
				.await
				.map(|(thread, _)| thread.id())
		};
		let first = enter(&[]).await.unwrap();
		let second = enter(&[("new", "true")]).await.unwrap();
		(first != second).xpect_true();
		enter(&[]).await.unwrap().xpect_eq(second);
		enter(&[("thread", &first.to_string())])
			.await
			.unwrap()
			.xpect_eq(first);
		enter(&[("thread", &ThreadId::from_u128(9).to_string())])
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("holds no thread");
	}

	/// A branch enters with its parent's history up to the post, reading no
	/// seed, and writes its own posts to its own thread.
	#[beet_core::test]
	async fn a_branch_continues_its_parent() {
		let store = BlobStore::temp();
		let (parent, records) = enter(
			&store,
			mount("threads"),
			Post::spawn("be brief"),
			request(&[]),
		)
		.await
		.unwrap();
		let point = PostRef::new(parent.id(), records.posts()[0].id());
		// a second entry joins the parent after the branch point
		enter(
			&store,
			mount("threads"),
			Post::spawn("unread"),
			request(&[]),
		)
		.await
		.unwrap();
		let (branch, records) = enter(
			&store,
			mount("threads"),
			SeedSource::blob(RelPath::new("seeds/missing.md")),
			request(&[("branch-of", &point.to_string())]),
		)
		.await
		.unwrap();
		branch.branch_of().xpect_eq(Some(&point));
		(branch.id() != parent.id()).xpect_true();
		texts(&records).xpect_eq(vec!["be brief".to_string()]);
		// the branch's join is its own, the parent's posts untouched
		let threads =
			BlobThreadStore::new(store.with_subdir(RelPath::new("threads")));
		threads
			.thread_posts(branch.id(), None)
			.await
			.unwrap()
			.iter()
			.all(|post| post.thread() == branch.id())
			.xpect_true();
		threads
			.thread_posts(parent.id(), None)
			.await
			.unwrap()
			.len()
			.xpect_eq(3);
	}

	/// A branch point the mount's store lacks is refused, as is a branch
	/// beside `--thread`.
	#[beet_core::test]
	async fn a_branch_is_refused() {
		let store = BlobStore::temp();
		let (parent, _) =
			enter(&store, mount("threads"), Post::spawn("hi"), request(&[]))
				.await
				.unwrap();
		let enter = async |params: &[(&str, &str)]| {
			enter(&store, mount("threads"), Post::spawn("hi"), request(params))
				.await
				.unwrap_err()
				.to_string()
		};
		let lost_post = PostRef::new(parent.id(), PostId::from_u128(7));
		enter(&[("branch-of", &lost_post.to_string())])
			.await
			.xpect_contains("names a post the thread");
		let lost_thread =
			PostRef::new(ThreadId::from_u128(7), PostId::from_u128(7));
		enter(&[("branch-of", &lost_thread.to_string())])
			.await
			.xpect_contains("names a thread the mount `threads` lacks");
		enter(&[
			("branch-of", &lost_post.to_string()),
			("thread", &parent.id().to_string()),
		])
		.await
		.xpect_contains("cannot continue `--thread`");
	}
}
