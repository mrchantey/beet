//! The store a thread's records persist in, see [`ThreadStore`].
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::exports::bytes::Bytes;
use beet_net::prelude::*;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

/// The stored threads of one directory, the truth every window of them is
/// open onto, in the table store declared at or above the mount.
///
/// A thread is a partition of that store, named by its [`ThreadId`], its own
/// isolated database holding the `thread`, `actor`, `post` and
/// `response_meta` tables ([`Thread`], [`Actor`], [`Post`],
/// [`ResponseMeta`]), each row keyed by its bare id. The root partition holds
/// the `thread_index`, one row a thread keyed `{dir}//{thread}`, so a
/// directory's own threads are contiguous apart from a nested directory's and
/// its newest is one reverse scan of one row. No query or atomic batch spans
/// two threads.
///
/// Creating a thread writes its actors, seed and thread row in one batch,
/// then its index row, so a listed thread is whole and a crash between the
/// two leaves an unlisted orphan, which no read lists. A store without
/// atomic batches and indexes is refused ([`Self::new`]).
///
/// A window's records flush into it as they change, each flush one atomic
/// batch of the posts it has not written and their metadata, a task of its
/// own; [`ThreadStore::flush`] awaits every one, so a window is torn down
/// only once its records are written. A `media` post's bytes are written to
/// `blobs/<cid>` in the nearest blob store before its row, so a stored
/// reference always resolves.
#[derive(Clone, Component)]
#[require(SyncedRecords)]
pub struct ThreadStore {
	/// The root partition of the table store holding every thread.
	root: TableStore,
	/// Where a `media` post's bytes are written, `None` to hold them in memory.
	blobs: Option<BlobStore>,
	/// The mount's filled directory, the prefix of every index key.
	dir: RelPath,
	/// The flushes spawned and not yet written.
	in_flight: Arc<AtomicUsize>,
}

impl ThreadStore {
	/// The threads kept under `dir` in `root`.
	///
	/// ## Errors
	///
	/// Errors when `root` lacks atomic batches or indexes, naming it and the
	/// fix.
	pub fn new(root: TableStore, dir: RelPath) -> Result<Self> {
		for capability in [Capability::AtomicApply, Capability::Indexes] {
			if !root.supports(capability) {
				bevybail!(
					"a thread's store needs {capability}, which `{}` lacks: declare one that has them, ie `<FsStore path=\"..\" {{JsonTableStore}}/>` or `<SqliteStore path=\"..\"/>`",
					root.describe()
				);
			}
		}
		Self {
			root,
			blobs: None,
			dir,
			in_flight: default(),
		}
		.xok()
	}

	/// Write `media` posts' bytes to `blobs`.
	pub fn with_blobs(mut self, blobs: BlobStore) -> Self {
		self.blobs = Some(blobs);
		self
	}

	/// The directory this store's threads are listed under.
	pub fn dir(&self) -> &RelPath { &self.dir }

	/// The blob store `media` posts' bytes are written to, if any.
	pub fn blobs(&self) -> Option<&BlobStore> { self.blobs.as_ref() }

	/// Ensure the backing store exists, creating it if needed.
	pub async fn try_create(&self) -> Result {
		self.root.store_try_create().await
	}

	/// Every thread listed in this directory, by id, never one of a nested
	/// directory.
	pub async fn threads(&self) -> Result<Vec<Thread>> {
		let mut threads = Vec::new();
		for id in self.listed(Scan::default()).await? {
			if let Some(thread) = self.read_thread(id).await? {
				threads.push(thread);
			}
		}
		threads.xok()
	}

	/// The newest thread listed in this directory, the one an entry
	/// continues: one reverse scan of one index row.
	pub async fn newest(&self) -> Result<Option<Thread>> {
		match self
			.listed(Scan::default().reverse().limit(1))
			.await?
			.first()
		{
			Some(id) => self.read_thread(*id).await,
			None => Ok(None),
		}
	}

	/// The thread `id`, if this directory lists it.
	pub async fn thread(&self, id: ThreadId) -> Result<Option<Thread>> {
		match self.index().exists(self.index_key(id)).await? {
			true => self.read_thread(id).await,
			false => Ok(None),
		}
	}

	/// The posts of `thread` alone, by [`PostId`]: a branch's inherited
	/// posts are its parent's ([`Self::history`]).
	pub async fn posts(&self, thread: ThreadId) -> Result<Vec<Post>> {
		let mut posts =
			self.partition(thread)?.table::<Post>().get_all().await?;
		for post in &mut posts {
			post.set_thread(thread);
		}
		posts.xok()
	}

	/// The actors of `thread` alone.
	pub async fn actors(&self, thread: ThreadId) -> Result<Vec<Actor>> {
		self.partition(thread)?.table::<Actor>().get_all().await
	}

	/// Every post of `thread`'s history in [`PostId`] order: for a branch, its
	/// parent's history up to and including the post it branches from,
	/// recursively, then its own, each read from its own partition.
	///
	/// ## Errors
	///
	/// Errors when a branch names a thread the store lacks, or the branches
	/// form a cycle.
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
	/// actors of each thread of its lineage and the response metadata of its
	/// posts.
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
			let metas = self
				.partition(id)?
				.table::<ResponseMeta>()
				.get_all()
				.await?;
			for meta in metas {
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

	/// Create `flush`'s thread: its actors, posts, metadata and thread row
	/// in its partition, the thread row last, then its index row, so a listed
	/// thread is whole. A thread id another thread already took is minted
	/// again, renaming the thread of `flush`'s posts.
	#[cfg(any(test, feature = "action"))]
	pub(crate) async fn create(&self, flush: &mut ThreadFlush) -> Result {
		while self
			.partition(flush.thread.id())?
			.table::<Thread>()
			.exists(flush.thread.id())
			.await?
		{
			flush.remint();
		}
		let id = flush.thread.id();
		let partition = self.partition(id)?;
		let mut writes = flush.writes(&partition)?;
		writes.push(
			partition
				.table::<Thread>()
				.put_write(flush.thread.clone(), Precondition::Absent)?,
		);
		self.write_blobs(&flush.posts).await?;
		Self::apply(&partition, writes).await?;
		self.index()
			.put(ThreadIndex {
				key: self.index_key(id),
				thread: id,
			})
			.await?;
		Ok(())
	}

	/// Write `flush` into its thread's partition as one atomic batch, its
	/// bytes first.
	async fn write(&self, flush: ThreadFlush) -> Result {
		let partition = self.partition(flush.thread.id())?;
		let writes = flush.writes(&partition)?;
		self.write_blobs(&flush.posts).await?;
		Self::apply(&partition, writes).await
	}

	/// Write the window `entity`'s records it has not yet written, then wait
	/// out every flush already in flight, so once this returns the store holds
	/// every completed post of the records. A thread with no store has nothing
	/// to flush.
	pub async fn flush(entity: &AsyncEntity) -> Result {
		let id = entity.id();
		let Some((store, flush)) =
			entity
				.world()
				.with(move |world: &mut World| {
					let store = world.get::<ThreadStore>(id)?.clone();
					let flush = world.with_state::<Query<(
						&Thread,
						&ThreadRecords,
						&mut SyncedRecords,
					)>, _>(|mut query| {
						query.get_mut(id).ok().and_then(
							|(thread, records, mut synced)| {
								ThreadFlush::take(thread, records, &mut synced)
							},
						)
					});
					(store, flush).xsome()
				})
				.await
		else {
			return Ok(());
		};
		if let Some(flush) = flush {
			store.write(flush).await?;
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
			store.write(flush).await
		});
	}

	/// The partition of `thread`.
	fn partition(&self, thread: ThreadId) -> Result<TableStore> {
		self.root.partition(thread.to_string())
	}

	/// The directory index, in the root partition.
	fn index(&self) -> Table<ThreadIndex> { self.root.table() }

	/// The key listing `thread` in this directory, `{dir}//{thread}`: a
	/// directory never holds an empty segment, so no param spells the `//`.
	fn index_key(&self, thread: ThreadId) -> TableKey {
		format!("{}//{thread}", self.dir).into()
	}

	/// The ids this directory lists within `scan`'s paging, its bounds the
	/// directory's own rows.
	async fn listed(&self, scan: Scan) -> Result<Vec<ThreadId>> {
		// `0` is the byte after `/`, so the range holds `{dir}//..` alone
		let scan = scan
			.from(format!("{}//", self.dir))
			.before(format!("{}/0", self.dir));
		let limited = scan.limit.is_some();
		let rows = match limited {
			true => self.index().scan(scan).await?.rows,
			false => self.index().scan_all(scan).await?,
		};
		rows.into_iter()
			.map(|row| row.thread)
			.collect::<Vec<_>>()
			.xok()
	}

	/// The thread row of `id`'s partition, listed or not.
	async fn read_thread(&self, id: ThreadId) -> Result<Option<Thread>> {
		self.partition(id)?.table::<Thread>().get(id).await
	}

	/// Write each `media` post's bytes it holds to `blobs/<cid>`, addressed
	/// by content so the same bytes twice are one blob. Without a blob store
	/// the bytes stay in memory on the window.
	async fn write_blobs(&self, posts: &[Post]) -> Result {
		let Some(blobs) = &self.blobs else {
			return Ok(());
		};
		for post in posts {
			if let (Some(bytes), Some(AgentPost::Media(view))) =
				(post.bytes(), post.as_agent_post())
				&& let Some(blob) = view.blob()
			{
				let path = Self::blob_path(&blob);
				if !blobs.exists(&path).await? {
					blobs.insert(&path, bytes.clone()).await?;
				}
			}
		}
		Ok(())
	}

	/// Read the bytes of `blob` from the blob store.
	pub async fn read_blob(&self, blob: &BlobRef) -> Result<Bytes> {
		match &self.blobs {
			Some(blobs) => blobs.get(&Self::blob_path(blob)).await,
			None => bevybail!("no blob store holds the bytes of {}", blob.cid),
		}
	}

	/// Where the bytes of `blob` live, as a PDS keeps them.
	fn blob_path(blob: &BlobRef) -> RelPath {
		RelPath::new(format!("blobs/{}", blob.cid))
	}

	/// Apply `writes` as one batch, in chunks only past the backend's limit.
	/// A post already written, its stored cid its own, is a retry of an
	/// earlier flush and dropped; any other conflict is raised.
	async fn apply(
		partition: &TableStore,
		mut writes: Vec<TableWrite>,
	) -> Result {
		loop {
			let Err(err) = partition.apply(writes.clone()).await else {
				return Ok(());
			};
			match TableError::of(&err) {
				Some(TableError::Limit { limit, .. }) if *limit > 0 => {
					let limit = *limit;
					for chunk in writes.chunks(limit) {
						Box::pin(Self::apply(partition, chunk.to_vec()))
							.await?;
					}
					return Ok(());
				}
				Some(TableError::Conflict {
					key,
					found: Some(found),
					..
				}) => {
					let written = writes.iter().position(|write| {
						matches!(write, TableWrite::Put { row, .. }
							if row.key() == key && row.cid() == found)
					});
					match written {
						Some(index) => {
							writes.remove(index);
						}
						None => return Err(err),
					}
				}
				_ => return Err(err),
			}
		}
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
				self.read_thread(point.thread()).await?.ok_or_else(|| {
					bevyhow!(
						"thread {} branches from {point}, whose thread the store lacks",
						thread.id(),
					)
				})?;
			lineage.push((parent.id(), Some(point.post())));
			branch = parent.branch_of().cloned();
		}
		lineage.xok()
	}
}

#[cfg(test)]
impl ThreadStore {
	/// Create `thread` in this store holding `actors` and `posts`, as an
	/// entry creating it would.
	pub(crate) async fn store_thread(
		&self,
		thread: &Thread,
		actors: Vec<Actor>,
		posts: Vec<Post>,
	) -> Result {
		let mut records = ThreadRecords::new();
		for actor in actors {
			records.insert_actor(actor);
		}
		for post in posts {
			records.upsert_post(post);
		}
		let mut flush =
			ThreadFlush::all(thread, &records, &mut SyncedRecords::default());
		self.create(&mut flush).await
	}
}

/// A row of the directory index: the thread `{dir}//{thread}` lists.
#[derive(Clone, Serialize, Deserialize)]
struct ThreadIndex {
	#[serde(skip, default = "ThreadIndex::unkeyed")]
	key: TableKey,
	thread: ThreadId,
}

impl ThreadIndex {
	/// The key before [`TableStoreRow::from_row`] sets it.
	fn unkeyed() -> TableKey { TableKey::from("") }
}

impl TableStoreRow for ThreadIndex {
	fn table_name() -> SmolStr { "thread_index".into() }
	fn key(&self) -> TableKey { self.key.clone() }
	fn from_row(key: &TableKey, row: Row) -> Result<Self> {
		let mut index = row.into_serde::<Self>()?;
		index.key = key.clone();
		index.xok()
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

/// What a window's store already holds, or another window writes: posts
/// loaded, received or flushed, metadata written, and each actor as last
/// written, so a flush writes only the window's own new records.
#[derive(Debug, Default, Clone, Component)]
pub struct SyncedRecords {
	posts: HashSet<PostId>,
	metas: HashSet<PostId>,
	actors: HashMap<ActorId, Actor>,
}

impl SyncedRecords {
	/// Every record of `records` held already, ie as loaded.
	pub fn of(records: &ThreadRecords) -> Self {
		let mut synced = Self::default();
		synced.extend(records);
		synced
	}

	/// Mark every record of `records` held already.
	pub fn extend(&mut self, records: &ThreadRecords) {
		self.posts.extend(records.posts().iter().map(Post::id));
		self.metas.extend(records.metas().map(|meta| meta.post_id));
		self.actors.extend(
			records
				.actors()
				.iter()
				.map(|actor| (actor.id().clone(), actor.clone())),
		);
	}

	/// Mark every record of `changes`, another window's, held already: its
	/// author's window writes them.
	pub fn receive(&mut self, changes: &ThreadChanges) {
		for post in &changes.posts {
			self.insert(post.id());
		}
		for actor in &changes.actors {
			self.actors.insert(actor.id().clone(), actor.clone());
		}
	}

	pub fn contains(&self, id: PostId) -> bool { self.posts.contains(&id) }

	/// Mark the post `id` and its metadata held already.
	pub fn insert(&mut self, id: PostId) {
		self.posts.insert(id);
		self.metas.insert(id);
	}

	/// Mark the metadata of `id` unwritten, so the next flush writes it
	/// again.
	pub fn remove_meta(&mut self, id: PostId) { self.metas.remove(&id); }
}

/// The records a window holds that its store does not yet: its own completed
/// posts, the metadata of its posts not yet written, and the authors of
/// either that changed. A branch's inherited posts are its parent's, never
/// written again.
pub(crate) struct ThreadFlush {
	thread: Thread,
	actors: Vec<Actor>,
	posts: Vec<Post>,
	metas: Vec<ResponseMeta>,
}

impl ThreadFlush {
	/// What the records hold that is not yet written, marking it synced, or
	/// `None` when everything is. A post in progress stays in the records
	/// until it completes.
	pub(crate) fn take(
		thread: &Thread,
		records: &ThreadRecords,
		synced: &mut SyncedRecords,
	) -> Option<Self> {
		let posts = records
			.posts()
			.iter()
			.filter(|post| {
				post.thread() == thread.id()
					&& !post.in_progress()
					&& !synced.posts.contains(&post.id())
			})
			.cloned()
			.collect::<Vec<_>>();
		for post in &posts {
			synced.posts.insert(post.id());
		}
		// the metadata of written posts not yet written, ie a usage landing
		// after its post flushed
		let metas = records
			.metas()
			.filter(|meta| {
				synced.posts.contains(&meta.post_id)
					&& !synced.metas.contains(&meta.post_id)
			})
			.cloned()
			.collect::<Vec<_>>();
		for meta in &metas {
			synced.metas.insert(meta.post_id);
		}
		let mut actors = Vec::new();
		for post in &posts {
			if let Ok(actor) = records.actor(post.author())
				&& synced.actors.get(actor.id()) != Some(actor)
			{
				synced.actors.insert(actor.id().clone(), actor.clone());
				actors.push(actor.clone());
			}
		}
		if posts.is_empty() && metas.is_empty() {
			return None;
		}
		Self {
			thread: thread.clone(),
			actors,
			posts,
			metas,
		}
		.xsome()
	}

	/// Everything `records` holds of its own thread, for creating it.
	#[cfg(any(test, feature = "action"))]
	pub(crate) fn all(
		thread: &Thread,
		records: &ThreadRecords,
		synced: &mut SyncedRecords,
	) -> Self {
		Self::take(thread, records, synced).unwrap_or_else(|| Self {
			thread: thread.clone(),
			actors: Vec::new(),
			posts: Vec::new(),
			metas: Vec::new(),
		})
	}

	/// The thread this flush writes.
	#[cfg(any(test, feature = "action"))]
	pub(crate) fn thread(&self) -> &Thread { &self.thread }

	/// Mint the thread a new id, its posts following it.
	#[cfg(any(test, feature = "action"))]
	fn remint(&mut self) {
		self.thread.remint();
		for post in &mut self.posts {
			post.set_thread(self.thread.id());
		}
	}

	/// The writes into `partition`: each actor put, each post absent, each
	/// meta put, since a response's usage may land after its post.
	fn writes(&self, partition: &TableStore) -> Result<Vec<TableWrite>> {
		let actors = partition.table::<Actor>();
		let posts = partition.table::<Post>();
		let metas = partition.table::<ResponseMeta>();
		let mut writes = Vec::new();
		for actor in &self.actors {
			writes.push(actors.put_write(actor.clone(), Precondition::Any)?);
		}
		for post in &self.posts {
			writes.push(posts.put_write(post.clone(), Precondition::Absent)?);
		}
		for meta in &self.metas {
			writes.push(metas.put_write(meta.clone(), Precondition::Any)?);
		}
		writes.xok()
	}
}

/// Flush each window's records to its [`ThreadStore`] whenever they change.
///
/// Only records not yet in [`SyncedRecords`] are written ([`ThreadFlush`]).
/// Each flush is a task counted on its store, which [`ThreadStore::flush`]
/// awaits. A window without a [`ThreadStore`] is ignored.
pub(crate) fn sync_records_to_store(
	async_commands: AsyncCommands,
	mut changed: Query<
		(&Thread, &ThreadRecords, &ThreadStore, &mut SyncedRecords),
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
	use super::ThreadFlush;
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// A completed text post by `author` in `thread`.
	fn post(author: &ActorId, thread: ThreadId, text: &str) -> Post {
		AgentPost::new_text(author.clone(), thread, text, PostStatus::Completed)
	}

	/// The text of each of `posts`.
	fn texts(posts: &[Post]) -> Vec<String> {
		posts.iter().map(|post| post.to_string()).collect()
	}

	/// Create `thread` in `store` holding `posts` by `actor`.
	async fn create(
		store: &ThreadStore,
		thread: &Thread,
		actor: &Actor,
		posts: Vec<Post>,
	) {
		store
			.store_thread(thread, vec![actor.clone()], posts)
			.await
			.unwrap();
	}

	/// Append `posts` to the stored `thread`, as a window's flush.
	async fn append(store: &ThreadStore, thread: &Thread, posts: Vec<Post>) {
		let mut records = ThreadRecords::new();
		for post in posts {
			records.upsert_post(post);
		}
		let flush =
			ThreadFlush::take(thread, &records, &mut SyncedRecords::default())
				.unwrap();
		store.write(flush).await.unwrap();
	}

	/// Exercises a [`ThreadStore`] over `table`: directories, ordering,
	/// actors and metadata per thread, branches, two writers on one thread,
	/// a retried flush and an orphan.
	pub async fn run(table: TableStore) {
		let store =
			ThreadStore::new(table.clone(), RelPath::new("threads/a")).unwrap();
		store.try_create().await.unwrap();
		let actor = Actor::user();
		let actor_id = actor.id().clone();

		// a thread of three posts, created out of order
		let thread = Thread::new("Conformance");
		let posts = (0..3)
			.map(|i| post(&actor_id, thread.id(), &format!("post {i}")))
			.collect::<Vec<_>>();
		let mut shuffled = posts.clone();
		shuffled.reverse();
		create(&store, &thread, &actor, shuffled).await;

		// reads come back id ordered
		texts(&store.posts(thread.id()).await.unwrap())
			.xpect_eq(vec!["post 0", "post 1", "post 2"]);
		store.actors(thread.id()).await.unwrap().len().xpect_eq(1);
		store.posts(ThreadId::now()).await.unwrap().xpect_empty();

		// a directory lists its own threads, never a nested or a sibling
		// directory's, though they share its name as a prefix
		for dir in ["threads/a/nested", "threads/ab"] {
			let other = Thread::new(dir);
			create(
				&ThreadStore::new(table.clone(), RelPath::new(dir)).unwrap(),
				&other,
				&actor,
				vec![post(&actor_id, other.id(), dir)],
			)
			.await;
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
		store.thread(ThreadId::now()).await.unwrap().xpect_none();

		// the newest thread is the index's last row, a nested directory's
		// newer thread beside it never counting
		let newer = Thread::new("Newer");
		create(&store, &newer, &actor, vec![post(
			&actor_id,
			newer.id(),
			"newer",
		)])
		.await;
		let nested = Thread::new("Nested and newest");
		create(
			&ThreadStore::new(table.clone(), RelPath::new("threads/a/z"))
				.unwrap(),
			&nested,
			&actor,
			vec![],
		)
		.await;
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
			read: Some(posts[1].id()),
		};
		let mut records = store.load_records(&thread).await.unwrap();
		let mut synced = SyncedRecords::of(&records);
		records.set_meta(meta.clone());
		synced.remove_meta(meta.post_id);
		store
			.write(ThreadFlush::take(&thread, &records, &mut synced).unwrap())
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
			let branch = Thread::new(text).with_branch_of(point);
			create(&store, &branch, &actor, vec![post(
				&actor_id,
				branch.id(),
				text,
			)])
			.await;
			texts(&store.history(&branch).await.unwrap())
				.xpect_eq(vec!["post 0", "post 1", text]);
			let records = store.load_records(&branch).await.unwrap();
			records.posts().len().xpect_eq(3);
			records.actor(&actor_id).unwrap();
			records.meta(posts[2].id()).xpect_none();
		}

		// two stores on one directory, as two runtimes, each write rows of
		// their own and never erase the other's
		let other =
			ThreadStore::new(table.clone(), RelPath::new("threads/a")).unwrap();
		let (mine, theirs) = (
			post(&actor_id, thread.id(), "mine"),
			post(&actor_id, thread.id(), "theirs"),
		);
		append(&store, &thread, vec![mine.clone()]).await;
		append(&other, &thread, vec![theirs]).await;
		texts(&store.posts(thread.id()).await.unwrap())
			.xpect_eq(vec!["post 0", "post 1", "post 2", "mine", "theirs"]);

		// a flush retried after it landed writes nothing twice
		append(&store, &thread, vec![mine.clone()]).await;
		store.posts(thread.id()).await.unwrap().len().xpect_eq(5);
		// another post at that id is a conflict, raised
		let mut forged = post(&actor_id, thread.id(), "forged");
		forged =
			Post::from_row(&mine.key(), forged.into_row().unwrap()).unwrap();
		forged.set_thread(thread.id());
		let mut records = ThreadRecords::new();
		records.upsert_post(forged);
		store
			.write(
				ThreadFlush::take(
					&thread,
					&records,
					&mut SyncedRecords::default(),
				)
				.unwrap(),
			)
			.await
			.unwrap_err();

		// a partition whose index row never landed is an orphan no read lists
		let orphan = Thread::new("Orphan");
		let partition = table.partition(orphan.id().to_string()).unwrap();
		partition
			.table::<Thread>()
			.put(orphan.clone())
			.await
			.unwrap();
		store.thread(orphan.id()).await.unwrap().xpect_none();
		store
			.threads()
			.await
			.unwrap()
			.iter()
			.any(|thread| thread.id() == orphan.id())
			.xpect_false();
	}

	#[beet_core::test]
	async fn memory() { run(TableStore::new(InMemoryTableStore::new())).await; }

	#[beet_core::test(timeout_ms = 30000)]
	async fn json() {
		let dir = TempDir::new_ws().unwrap();
		run(JsonTableStore::over(BlobStore::new(FsStore::new(
			dir.path().clone(),
		)))
		.unwrap())
		.await;
	}

	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test(timeout_ms = 30000)]
	async fn sqlite_files() {
		let dir = TempDir::new_ws().unwrap();
		run(TableStore::new(SqliteStore::new(dir.join("threads.db")))).await;
	}

	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test(timeout_ms = 30000)]
	async fn sqlite_memory() {
		run(TableStore::new(SqliteStore::memory("thread-suite"))).await;
	}

	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test(timeout_ms = 60000)]
	async fn partition_host() {
		let host = PartitionHost::memory("thread-suite-host");
		run(TableStore::new(host.client().await.unwrap())).await;
	}

	/// The suite against DynamoDB Local
	/// (`podman run -p 8000:8000 amazon/dynamodb-local`).
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test(timeout_ms = 120_000)]
	#[ignore = "needs DynamoDB Local on 127.0.0.1:8000"]
	async fn dynamo() {
		// a table of its own each run, so a reused container holds no rows
		run(TableStore::new(
			DynamoStore::new(
				format!("beet-thread-{}", Tid::now()),
				"us-west-2",
			)
			.with_endpoint("http://127.0.0.1:8000"),
		))
		.await;
	}

	/// A `media` post's row holds a reference, its bytes landing once at
	/// `blobs/<cid>` however many posts hold them, and read back from there.
	#[beet_core::test]
	async fn bytes_are_stored_by_reference() {
		let blobs = BlobStore::temp();
		let store = ThreadStore::new(
			TableStore::new(InMemoryTableStore::new()),
			RelPath::new("threads"),
		)
		.unwrap()
		.with_blobs(blobs.clone());
		let thread = Thread::new("Photos");
		let photo = || {
			AgentPost::new_media(
				ActorId::USER,
				thread.id(),
				MediaType::Png,
				vec![1, 2, 3],
				None,
				PostStatus::Completed,
			)
		};
		store
			.store_thread(&thread, vec![Actor::user()], vec![photo(), photo()])
			.await
			.unwrap();
		let stored = blobs.get_all().await.unwrap();
		stored.len().xpect_eq(1);
		stored[0].0.to_string().xpect_starts_with("blobs/");
		let posts = store.posts(thread.id()).await.unwrap();
		posts.len().xpect_eq(2);
		for post in &posts {
			post.bytes().xpect_none();
			let Some(AgentPost::Media(view)) = post.as_agent_post() else {
				panic!("expected a media post, found {post:?}");
			};
			store
				.read_blob(&view.blob().unwrap())
				.await
				.unwrap()
				.to_vec()
				.xpect_eq(vec![1, 2, 3]);
		}
	}

	/// A store lacking atomic batches or indexes is refused, naming the fix.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	fn refuses_a_store_without_batches() {
		// a client that never asked its server knows of no capability
		let client = TableWireClient::new(
			PartitionHost::memory("thread-refusal-host").transport(),
		);
		ThreadStore::new(TableStore::new(client), RelPath::new("threads"))
			.err()
			.unwrap()
			.to_string()
			.xpect_contains("JsonTableStore");
	}
}
