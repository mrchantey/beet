//! The store a thread's records persist in, see [`ThreadStore`].
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

/// The stored threads of one directory, the truth every window of them is a
/// view onto: [`Thread`], [`Actor`], [`Post`] and [`ResponseMeta`] rows in a
/// [`TableStore`].
///
/// Every record is its own row, keyed by the mount's filled directory, its
/// thread and its id, so the key is what scopes a thread and no backend need
/// honour a subdirectory, as SQLite's own tables do not:
///
/// | table                  | key                       |
/// | ---------------------- | ------------------------- |
/// | `thread`               | `{dir}/{thread}`          |
/// | `thread_actor`         | `{dir}/{thread}/{actor}`  |
/// | `thread_post`          | `{dir}/{thread}/{post}`   |
/// | `thread_response_meta` | `{dir}/{thread}/{post}`   |
///
/// One thread's rows load by key, the rows nested under `{dir}/{thread}`
/// ([`Scan::under`]), never reading another thread's, and the newest thread
/// of a directory is the one with the greatest time ordered id. A write is a
/// row put, never a rewrite of the thread, so two writers on one thread
/// never erase each other.
///
/// A window's records flush into it as they change, each flush a task of its
/// own; [`ThreadStore::flush`] awaits every one, so a window is torn down only
/// once its records are written.
#[derive(Clone, Component)]
#[require(SyncedPosts)]
pub struct ThreadStore {
	/// The table store holding every directory's rows.
	table: TableStore,
	/// The mount's filled directory, the prefix of every key.
	dir: RelPath,
	/// The flushes spawned and not yet written.
	in_flight: Arc<AtomicUsize>,
}

impl ThreadStore {
	/// The threads kept under `dir` in `table`.
	pub fn new(table: TableStore, dir: RelPath) -> Self {
		Self {
			table,
			dir,
			in_flight: default(),
		}
	}

	/// The directory this store's threads are kept under.
	pub fn dir(&self) -> &RelPath { &self.dir }

	/// Ensure the backing store exists, creating it if needed.
	pub async fn try_create(&self) -> Result {
		self.rows::<Thread>().store_try_create().await
	}

	/// Every thread stored directly in this directory, by id, never one of a
	/// nested directory.
	pub async fn threads(&self) -> Result<Vec<Thread>> {
		let mut threads = self.direct_threads().await?;
		threads.sort_by_key(|thread| thread.id());
		threads.xok()
	}

	/// The newest thread of this directory, the one an entry continues, found
	/// by its time ordered id alone.
	pub async fn newest(&self) -> Result<Option<Thread>> {
		self.direct_threads()
			.await?
			.into_iter()
			.max_by_key(|thread| thread.id())
			.xok()
	}

	/// The stored thread `id`, if this directory holds it.
	pub async fn thread(&self, id: ThreadId) -> Result<Option<Thread>> {
		self.rows::<Thread>()
			.get(self.key(id))
			.await?
			.map(|row| row.record)
			.xok()
	}

	/// The posts of `thread` alone, by [`PostId`]: a branch's inherited
	/// posts are its parent's ([`Self::history`]).
	pub async fn posts(&self, thread: ThreadId) -> Result<Vec<Post>> {
		let mut posts = self.thread_rows::<Post>(thread).await?;
		posts.sort_by_key(|post| post.id());
		posts.xok()
	}

	/// The actors that authored the posts of `thread` alone.
	pub async fn actors(&self, thread: ThreadId) -> Result<Vec<Actor>> {
		self.thread_rows::<Actor>(thread).await
	}

	/// Every post of `thread`'s history in [`PostId`] order: for a branch, its
	/// parent's history up to and including the post it branches from,
	/// recursively, then its own. A branch references its parent's posts,
	/// never copies them.
	///
	/// ## Errors
	///
	/// Errors when a branch names a thread the directory lacks, or the
	/// branches form a cycle.
	pub async fn history(&self, thread: &Thread) -> Result<Vec<Post>> {
		let mut posts = Vec::new();
		for (id, until) in self.lineage(thread).await? {
			posts.extend(
				self.posts(id).await?.into_iter().filter(|post| {
					until.is_none_or(|until| post.id() <= until)
				}),
			);
		}
		posts.sort_by_key(|post| post.id());
		posts.xok()
	}

	/// The records `thread` enters with: its whole [`Self::history`], the
	/// actors that authored it and the response metadata of its posts.
	pub async fn load_records(&self, thread: &Thread) -> Result<ThreadRecords> {
		let mut records = ThreadRecords::new();
		let mut posts = Vec::new();
		for (id, until) in self.lineage(thread).await? {
			for actor in self.actors(id).await? {
				records.insert_actor(actor);
			}
			let kept = self
				.posts(id)
				.await?
				.into_iter()
				.filter(|post| until.is_none_or(|until| post.id() <= until))
				.collect::<Vec<_>>();
			for meta in self.thread_rows::<ResponseMeta>(id).await? {
				if kept.iter().any(|post| post.id() == meta.post_id) {
					records.set_meta(meta);
				}
			}
			posts.extend(kept);
		}
		let mut stored = ThreadRecords::new();
		for post in posts {
			stored.upsert_post(post);
		}
		records.merge_stored(stored);
		records.xok()
	}

	/// Write `thread`'s row, then a row for each actor, post and meta, each
	/// replacing any at its key.
	pub(crate) async fn insert(
		&self,
		thread: &Thread,
		actors: Vec<Actor>,
		posts: Vec<Post>,
		metas: Vec<ResponseMeta>,
	) -> Result {
		let id = thread.id();
		self.rows::<Thread>()
			.put(RecordRow::new(self.key(id), thread.clone()))
			.await?;
		self.push_thread_rows(id, actors, |actor| actor.id().to_string())
			.await?;
		self.push_thread_rows(id, posts, |post| post.id().to_string())
			.await?;
		self.push_thread_rows(id, metas, |meta| meta.post_id.to_string())
			.await
	}

	/// Write the thread `entity`'s completed posts not yet flushed, then wait
	/// out every flush already in flight, so once this returns the store holds
	/// every completed post of the records. A thread with no store has nothing
	/// to flush.
	pub async fn flush(entity: &AsyncEntity) -> Result {
		let id = entity.id();
		let Some((store, flush)) = entity
			.world()
			.with(move |world: &mut World| {
				let store = world.get::<ThreadStore>(id)?.clone();
				let flush =
					world
						.with_state::<Query<(&Thread, &ThreadRecords, &mut SyncedPosts)>, _>(
							|mut query| {
								query.get_mut(id).ok().and_then(
									|(thread, records, mut synced)| {
										ThreadFlush::take(
											thread,
											records,
											&mut synced,
										)
									},
								)
							},
						);
				(store, flush).xsome()
			})
			.await
		else {
			return Ok(());
		};
		if let Some(flush) = flush {
			flush.write(&store).await?;
		}
		while store.in_flight.load(Ordering::SeqCst) > 0 {
			async_ext::yield_now().await;
			entity.world().with(|_| ()).await;
		}
		Ok(())
	}

	/// Spawn `flush` as a task of its own, counted until it is written.
	fn spawn_flush(&self, commands: &AsyncCommands, flush: ThreadFlush) {
		let store = self.clone();
		let in_flight = InFlight::new(self.in_flight.clone());
		commands.run(async move |_: AsyncWorld| -> Result {
			let _in_flight = in_flight;
			flush.write(&store).await
		});
	}

	/// The table of `T` rows.
	fn rows<T: RowRecord>(&self) -> Table<RecordRow<T>> { self.table.table() }

	/// The key `{dir}/{segment}`.
	fn key(&self, segment: impl std::fmt::Display) -> TableKey {
		self.dir.join(&segment.to_string()).to_string().into()
	}

	/// Every thread stored directly in this directory, in no order.
	async fn direct_threads(&self) -> Result<Vec<Thread>> {
		self.rows::<Thread>()
			.scan_all(Scan::default().under(self.dir.to_string()))
			.await?
			.into_iter()
			// a nested directory's rows share the prefix
			.filter(|row| row.key == self.key(row.record.id()))
			.map(|row| row.record)
			.collect::<Vec<_>>()
			.xok()
	}

	/// Every `T` row nested under `thread`'s key.
	async fn thread_rows<T: RowRecord>(
		&self,
		thread: ThreadId,
	) -> Result<Vec<T>> {
		self.rows::<T>()
			.scan_all(Scan::default().under(self.key(thread).as_str()))
			.await?
			.into_iter()
			.map(|row| row.record)
			.collect::<Vec<_>>()
			.xok()
	}

	/// Write each of `records` under `thread`'s key, `id` naming its row.
	async fn push_thread_rows<T: RowRecord>(
		&self,
		thread: ThreadId,
		records: Vec<T>,
		id: impl Fn(&T) -> String,
	) -> Result {
		if records.is_empty() {
			return Ok(());
		}
		let parent = self.key(thread);
		let rows = self.rows::<T>();
		records
			.into_iter()
			.map(|record| {
				let key = format!("{parent}/{}", id(&record));
				rows.put(RecordRow::new(key.into(), record))
			})
			.xmap(|puts| {
				async_ext::try_join_all_bounded(
					BlobStore::GET_ALL_CONCURRENCY,
					puts,
				)
			})
			.await?;
		Ok(())
	}

	/// `thread`, then each thread it branches from, each paired with the last
	/// post of it the history holds, `None` for all of them.
	async fn lineage(
		&self,
		thread: &Thread,
	) -> Result<Vec<(ThreadId, Option<PostId>)>> {
		let mut lineage = vec![(thread.id(), None)];
		let mut branch = thread.branch_of().cloned();
		while let Some(point) = branch {
			if lineage.iter().any(|(id, _)| *id == point.thread()) {
				bevybail!("thread {} branches in a cycle", thread.id());
			}
			let parent =
				self.thread(point.thread()).await?.ok_or_else(|| {
					bevyhow!(
						"thread {} branches from {point}, whose thread the directory `{}` lacks",
						thread.id(),
						self.dir
					)
				})?;
			lineage.push((parent.id(), Some(point.post())));
			branch = parent.branch_of().cloned();
		}
		lineage.xok()
	}
}

/// A record stored as a row of its own table.
trait RowRecord: TableContent {
	/// The table its rows live in.
	const TABLE: &'static str;
}

impl RowRecord for Thread {
	const TABLE: &'static str = "thread";
}
impl RowRecord for Actor {
	const TABLE: &'static str = "thread_actor";
}
impl RowRecord for Post {
	const TABLE: &'static str = "thread_post";
}
impl RowRecord for ResponseMeta {
	const TABLE: &'static str = "thread_response_meta";
}

/// One record as a row: the record alone is stored, at the key the
/// [`ThreadStore`] writing it computed.
#[derive(Clone)]
struct RecordRow<T> {
	key: TableKey,
	record: T,
}

impl<T> RecordRow<T> {
	fn new(key: TableKey, record: T) -> Self { Self { key, record } }
}

impl<T: Serialize> Serialize for RecordRow<T> {
	fn serialize<S: serde::Serializer>(
		&self,
		serializer: S,
	) -> Result<S::Ok, S::Error> {
		self.record.serialize(serializer)
	}
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for RecordRow<T> {
	fn deserialize<D: serde::Deserializer<'de>>(
		deserializer: D,
	) -> Result<Self, D::Error> {
		T::deserialize(deserializer).map(|record| Self::new("".into(), record))
	}
}

impl<T: RowRecord> TableStoreRow for RecordRow<T> {
	fn table_name() -> SmolStr { T::TABLE.into() }
	fn key(&self) -> TableKey { self.key.clone() }
	fn from_row(key: &TableKey, row: Row) -> Result<Self> {
		Self::new(key.clone(), row.into_serde()?).xok()
	}
}

/// One counted in-flight flush, released however its task ends.
struct InFlight(Arc<AtomicUsize>);

impl InFlight {
	fn new(count: Arc<AtomicUsize>) -> Self {
		count.fetch_add(1, Ordering::SeqCst);
		Self(count)
	}
}

impl Drop for InFlight {
	fn drop(&mut self) { self.0.fetch_sub(1, Ordering::SeqCst); }
}

/// The posts of a window already in its [`ThreadStore`], or another
/// window's to write: loaded history, received posts and each flushed one,
/// so a flush writes only the window's own new posts.
#[derive(Debug, Default, Clone, Component)]
pub struct SyncedPosts(HashSet<PostId>);

impl SyncedPosts {
	pub fn new(synced: HashSet<PostId>) -> Self { Self(synced) }
	pub fn contains(&self, id: PostId) -> bool { self.0.contains(&id) }
	pub fn insert(&mut self, id: PostId) { self.0.insert(id); }
	/// Mark `id` unflushed, so the next flush writes it again.
	pub fn remove(&mut self, id: PostId) { self.0.remove(&id); }
}

/// The records a window holds that its store does not yet: its own completed
/// posts not in [`SyncedPosts`], their authors and response metadata. A
/// branch's inherited posts are its parent's, never written again.
struct ThreadFlush {
	thread: Thread,
	actors: Vec<Actor>,
	posts: Vec<Post>,
	metas: Vec<ResponseMeta>,
}

impl ThreadFlush {
	/// The unflushed completed posts of the records, marking them synced, or
	/// `None` when every completed post is already flushed. A post in
	/// progress stays in the records until it completes.
	fn take(
		thread: &Thread,
		records: &ThreadRecords,
		synced: &mut SyncedPosts,
	) -> Option<Self> {
		let posts = records
			.posts()
			.iter()
			.filter(|post| {
				post.thread() == thread.id()
					&& !post.in_progress()
					&& !synced.contains(post.id())
			})
			.cloned()
			.collect::<Vec<_>>();
		if posts.is_empty() {
			return None;
		}
		// dedup the authors and collect metadata for the new posts
		let mut actors: Vec<Actor> = Vec::new();
		let mut metas: Vec<ResponseMeta> = Vec::new();
		for post in &posts {
			if let Ok(actor) = records.actor(post.author())
				&& !actors.iter().any(|existing| existing.id() == actor.id())
			{
				actors.push(actor.clone());
			}
			if let Some(meta) = records.meta(post.id()) {
				metas.push(meta.clone());
			}
			synced.insert(post.id());
		}
		Self {
			thread: thread.clone(),
			actors,
			posts,
			metas,
		}
		.xsome()
	}

	/// Write the rows, the thread's first.
	async fn write(self, store: &ThreadStore) -> Result {
		store
			.insert(&self.thread, self.actors, self.posts, self.metas)
			.await
	}
}

/// Flush each window's records to its [`ThreadStore`] whenever they change.
///
/// Only completed posts not yet in [`SyncedPosts`] are written, with their
/// authors and any response metadata ([`ThreadFlush`]). Each flush is a task
/// counted on its store, which [`ThreadStore::flush`] awaits. A window without
/// a [`ThreadStore`] is ignored.
pub(crate) fn sync_records_to_store(
	async_commands: AsyncCommands,
	mut changed: Query<
		(&Thread, &ThreadRecords, &ThreadStore, &mut SyncedPosts),
		Changed<ThreadRecords>,
	>,
) -> Result {
	for (thread, records, store, mut synced) in changed.iter_mut() {
		if let Some(flush) = ThreadFlush::take(thread, records, &mut synced) {
			store.spawn_flush(&async_commands, flush);
		}
	}
	Ok(())
}

/// The suite every backend of a [`TableStore`] passes as a thread's store.
#[cfg(test)]
pub(crate) mod thread_store_test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// A completed text post by `author` in `thread`.
	fn post(author: ActorId, thread: ThreadId, text: &str) -> Post {
		AgentPost::new_text(author, thread, text, PostStatus::Completed)
	}

	/// The text of each of `posts`.
	fn texts(posts: &[Post]) -> Vec<String> {
		posts.iter().map(|post| post.to_string()).collect()
	}

	/// Exercises a [`ThreadStore`] over `table`: directories, ordering,
	/// actors and metadata per thread, branches, and two writers on one
	/// thread.
	pub async fn run(table: TableStore) {
		let store = ThreadStore::new(table.clone(), RelPath::new("threads/a"));
		store.try_create().await.unwrap();
		let actor = Actor::user();
		let actor_id = actor.id();

		// a thread of three posts, inserted out of order
		let thread = Thread::new("Conformance");
		let posts = (0..3)
			.map(|i| post(actor_id, thread.id(), &format!("post {i}")))
			.collect::<Vec<_>>();
		let mut shuffled = posts.clone();
		shuffled.reverse();
		store
			.insert(&thread, vec![actor.clone()], shuffled, vec![])
			.await
			.unwrap();

		// reads come back id ordered
		texts(&store.posts(thread.id()).await.unwrap())
			.xpect_eq(vec!["post 0", "post 1", "post 2"]);
		store.actors(thread.id()).await.unwrap().len().xpect_eq(1);
		store
			.posts(ThreadId::new_now())
			.await
			.unwrap()
			.xpect_empty();

		// a directory holds its own threads, never a nested or a sibling
		// directory's, though they share its name as a prefix
		for dir in ["threads/a/nested", "threads/ab"] {
			let other = Thread::new(dir);
			ThreadStore::new(table.clone(), RelPath::new(dir))
				.insert(
					&other,
					vec![actor.clone()],
					vec![post(actor_id, other.id(), dir)],
					vec![],
				)
				.await
				.unwrap();
		}
		store
			.threads()
			.await
			.unwrap()
			.iter()
			.map(Thread::id)
			.collect::<Vec<_>>()
			.xpect_eq(vec![thread.id()]);
		store.thread(thread.id()).await.unwrap().xpect_some();
		store
			.thread(ThreadId::new_now())
			.await
			.unwrap()
			.xpect_none();

		// the newest thread is found by id
		let newer = Thread::new("Newer");
		store
			.insert(
				&newer,
				vec![actor.clone()],
				vec![post(actor_id, newer.id(), "newer")],
				vec![],
			)
			.await
			.unwrap();
		store
			.newest()
			.await
			.unwrap()
			.unwrap()
			.id()
			.xpect_eq(newer.id());

		// metadata loads with its post, for `previous_response_id`
		let meta = ResponseMeta {
			post_id: posts[2].id(),
			provider_slug: "openai".into(),
			model_slug: "gpt-5-mini".into(),
			response_id: "resp_1".into(),
			response_stored: true,
			usage: None,
		};
		store
			.insert(&thread, vec![], vec![], vec![meta.clone()])
			.await
			.unwrap();
		store
			.load_records(&thread)
			.await
			.unwrap()
			.meta(posts[2].id())
			.xpect_eq(Some(&meta));

		// a branch of the second post holds the parent's history up to it,
		// then its own, and a sibling branch of that post is independent
		let point = PostRef::new(thread.id(), posts[1].id());
		for text in ["left", "right"] {
			let branch = Thread::new(text).with_branch_of(point.clone());
			store
				.insert(
					&branch,
					vec![],
					vec![post(actor_id, branch.id(), text)],
					vec![],
				)
				.await
				.unwrap();
			texts(&store.history(&branch).await.unwrap())
				.xpect_eq(vec!["post 0", "post 1", text]);
			let records = store.load_records(&branch).await.unwrap();
			records.posts().len().xpect_eq(3);
			records.actor(actor_id).unwrap();
			records.meta(posts[2].id()).xpect_none();
		}

		// two stores on one directory, as two runtimes, each write rows of
		// their own and never erase the other's
		let other = ThreadStore::new(table.clone(), RelPath::new("threads/a"));
		let (mine, theirs) = (
			post(actor_id, thread.id(), "mine"),
			post(actor_id, thread.id(), "theirs"),
		);
		store
			.insert(&thread, vec![actor.clone()], vec![mine], vec![])
			.await
			.unwrap();
		other
			.insert(&thread, vec![actor.clone()], vec![theirs], vec![])
			.await
			.unwrap();
		texts(&store.posts(thread.id()).await.unwrap())
			.xpect_eq(vec!["post 0", "post 1", "post 2", "mine", "theirs"]);
		store
			.load_records(&thread)
			.await
			.unwrap()
			.meta(posts[2].id())
			.xpect_some();
	}

	#[beet_core::test]
	async fn memory() { run(TableStore::temp()).await; }

	#[beet_core::test(timeout_ms = 30000)]
	async fn fs() {
		let dir = TempDir::new_ws().unwrap();
		run(TableStore::new(BlobStore::new(FsStore::new(
			dir.path().clone(),
		))))
		.await;
	}

	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test(timeout_ms = 30000)]
	async fn sqlite() {
		let dir = TempDir::new_ws().unwrap();
		run(TableStore::new(SqliteStore::new(dir.join("threads.db")))).await;
	}
}
