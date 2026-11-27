//! The store a thread's records persist in, see [`ThreadStore`].
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::exports::bytes::Bytes;
use beet_net::prelude::*;
use std::collections::BTreeSet;
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
/// A window opens on the newest of a thread's posts and reads older ones as
/// needed ([`Self::open`], [`Self::read_older`]), each read a range of keys
/// in one partition after another along a branch's lineage, with the actors
/// that authored them and their metadata, never a whole table.
///
/// A window's records flush into it as they change, each flush one atomic
/// batch of the posts it has not written and their metadata, a task of its
/// own; [`ThreadStore::flush`] awaits every one, so a window is torn down
/// only once its records are written. The bytes of a file a post holds are
/// written to `blobs/<cid>` in the nearest blob store before its row, so a
/// stored reference always resolves.
///
/// The store holds everything, a deleted post's body and an edit's original
/// included, and so does every read of it ([`Self::posts`], [`Self::open`]):
/// what a person or an agent reads is the records shown
/// ([`ThreadRecords::shown`]). An edit or a deletion is written by the window
/// making it, as one batch ([`EditPost`], [`DeletePost`]).
#[derive(Clone, Component)]
#[require(SyncedRecords)]
pub struct ThreadStore {
	/// The root partition of the table store holding every thread.
	root: TableStore,
	/// Where the bytes of a post's file are written, `None` to hold them in
	/// memory.
	blobs: Option<BlobStore>,
	/// The mount's filled directory, the prefix of every index key.
	dir: RelPath,
	/// How many posts a window without an agent opens on, and how many
	/// more each read of older posts takes.
	load: usize,
	/// The flushes spawned and not yet written.
	in_flight: Arc<AtomicUsize>,
}

/// How far back a window opens onto its thread ([`ThreadStore::open`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reach {
	/// Every post, for a window holding an agent, whose context is the
	/// whole thread until a compaction bounds it.
	Whole,
	/// The newest `count` posts, back to the last word at least, and the
	/// seed before them when `seed`, which a canonical window always holds.
	Newest {
		/// How many of the newest posts.
		count: usize,
		/// Whether the seed is held apart from the run.
		seed: bool,
	},
	/// Back through the newest compaction of each of `agents`, a window
	/// whose every agent compacts ([`Compact`](crate::prelude::Compact)):
	/// each agent's context, and the seed held apart; the whole thread when
	/// any of them has no compaction yet.
	Compacted {
		/// The agents whose contexts the window holds.
		agents: Vec<ActorId>,
	},
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
			load: MountThreadStore::DEFAULT_LOAD,
			in_flight: default(),
		}
		.xok()
	}

	/// Open windows on the newest `load` posts and read `load` more at a
	/// time.
	pub fn with_load(mut self, load: usize) -> Self {
		self.load = load.max(1);
		self
	}

	/// How many posts a window without an agent opens on, and how many more
	/// each read of older posts takes.
	pub fn load(&self) -> usize { self.load }

	/// Write the bytes of posts' files to `blobs`.
	pub fn with_blobs(mut self, blobs: BlobStore) -> Self {
		self.blobs = Some(blobs);
		self
	}

	/// The directory this store's threads are listed under.
	pub fn dir(&self) -> &RelPath { &self.dir }

	/// The blob store the bytes of posts' files are written to, if any.
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

	/// Every post of `thread` alone, by [`PostId`], its whole table, for a
	/// program reading everything, what was edited away and deleted
	/// included: a window reads only as far as it needs
	/// ([`Self::open`]), and a branch's inherited posts are its parent's.
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

	/// Whether `post` is in `thread`'s history: its own, or for a branch its
	/// parent's up to the branch point, recursively.
	///
	/// ## Errors
	///
	/// Errors when a branch names a thread the store lacks, or the branches
	/// form a cycle.
	pub async fn holds_post(
		&self,
		thread: &Thread,
		post: PostId,
	) -> Result<bool> {
		for (id, until) in self.lineage(thread).await? {
			if until.is_none_or(|until| post <= until)
				&& self.partition(id)?.table::<Post>().exists(post).await?
			{
				return Ok(true);
			}
		}
		Ok(false)
	}

	/// The post `id` of `thread`'s history, its own or for a branch its
	/// parent's up to the branch point, everything included.
	///
	/// ## Errors
	///
	/// Errors as [`Self::open`].
	pub async fn read_post(
		&self,
		thread: &Thread,
		id: PostId,
	) -> Result<Option<Post>> {
		for (partition, until) in self.lineage(thread).await? {
			if until.is_some_and(|until| id > until) {
				continue;
			}
			if let Some(mut post) =
				self.partition(partition)?.table::<Post>().get(id).await?
			{
				post.set_thread(partition);
				return post.xsome().xok();
			}
		}
		Ok(None)
	}

	/// Up to `count` posts of `thread`'s history from `from` on, oldest
	/// first, across a branch's lineage, with their authors and metadata:
	/// everything, a deleted post's body and an edit's original included.
	///
	/// ## Errors
	///
	/// Errors as [`Self::open`].
	pub async fn read_from(
		&self,
		thread: &Thread,
		from: PostId,
		count: usize,
	) -> Result<ThreadRecords> {
		let lineage = self.lineage(thread).await?;
		let mut posts = Vec::new();
		// the root's segment is the oldest, so read from it forward
		for (partition, until) in lineage.iter().rev() {
			if posts.len() >= count {
				break;
			}
			let mut scan = Scan::default().from(from);
			if let Some(until) = until {
				scan = scan.through(*until);
			}
			// a branch's own posts follow the point it left its parent at
			if let Some(after) = posts.last().map(Post::id) {
				scan = scan.after(after);
			}
			let mut rows = self
				.partition(*partition)?
				.table::<Post>()
				.scan_first(scan, count - posts.len())
				.await?;
			for post in &mut rows {
				post.set_thread(*partition);
			}
			posts.append(&mut rows);
		}
		self.records_of(&lineage, posts, None).await
	}

	/// The oldest of each of `agents`' newest compaction across `thread`'s
	/// lineage, found by the `by_kind` index, `None` when any of them has
	/// none. On a backend whose index lags, ie Dynamo, an older compaction
	/// may be found, which reads more posts, never fewer.
	///
	/// ## Errors
	///
	/// Errors as [`Self::open`].
	pub async fn oldest_newest_compaction(
		&self,
		thread: &Thread,
		agents: &[ActorId],
	) -> Result<Option<Post>> {
		let lineage = self.lineage(thread).await?;
		let mut oldest: Option<Post> = None;
		for agent in agents {
			let mut newest = None;
			for (partition, until) in &lineage {
				let mut scan = IndexScan::default()
					.from([
						Value::from(PostKind::Compaction.as_str()),
						Value::from(agent.as_str()),
					])
					.reverse()
					.limit(1);
				scan = match until {
					Some(until) => scan.through([
						Value::from(PostKind::Compaction.as_str()),
						Value::from(agent.as_str()),
						Value::from(until.to_string()),
					]),
					None => scan.through([
						Value::from(PostKind::Compaction.as_str()),
						Value::from(agent.as_str()),
					]),
				};
				let page = self
					.partition(*partition)?
					.table::<Post>()
					.scan_index("by_kind", scan)
					.await?;
				if let Some(mut post) = page.rows.into_iter().next() {
					post.set_thread(*partition);
					newest = Some(post);
					break;
				}
			}
			let Some(newest) = newest else {
				return Ok(None);
			};
			if oldest
				.as_ref()
				.is_none_or(|oldest| newest.id() < oldest.id())
			{
				oldest = Some(newest);
			}
		}
		oldest.xok()
	}

	/// The records a window opens `thread` with, as far back as `reach`:
	/// posts in [`PostId`] order across a branch's lineage, each read from
	/// its own partition, the actors that authored them and their metadata,
	/// and the run's start where older posts exist
	/// ([`ThreadRecords::has_older`]). The run reaches back to the last word
	/// at least, so a person's turn reads it ([`ThreadRecords::last_word`]).
	///
	/// ## Errors
	///
	/// Errors when a branch names a thread the store lacks, or the branches
	/// form a cycle.
	pub async fn open(
		&self,
		thread: &Thread,
		reach: Reach,
	) -> Result<ThreadRecords> {
		let lineage = self.lineage(thread).await?;
		// a compacting window reads back through each agent's newest
		// compaction, else the whole thread
		let (reach, stop) = match reach {
			Reach::Compacted { agents } => {
				match self.oldest_newest_compaction(thread, &agents).await? {
					Some(compaction) => (
						Reach::Newest {
							count: self.load,
							seed: true,
						},
						compaction.subject(),
					),
					None => (Reach::Whole, None),
				}
			}
			reach => (reach, None),
		};
		let (count, seed) = match reach {
			Reach::Newest { count, seed } => (Some(count.max(1)), seed),
			_ => (None, false),
		};
		let mut posts = match seed {
			true => self.seed_posts(&lineage).await?,
			false => Vec::new(),
		};
		let floor = posts.last().map(Post::id);
		let mut run = Vec::new();
		let mut more = true;
		// every post after the compaction's subject
		if let Some(stop) = stop {
			let (after, _) = self
				.posts_back(&lineage, None, floor.max(Some(stop)), None)
				.await?;
			run = after;
			more = !run.iter().any(Post::is_word);
		}
		// the newest `count`, then older runs of as many until one is said
		while more {
			let before = run.first().map(Post::id);
			let (mut older, has_more) =
				self.posts_back(&lineage, before, floor, count).await?;
			more = has_more;
			let said = older.iter().any(Post::is_word);
			older.append(&mut run);
			run = older;
			if said {
				break;
			}
		}
		// a compacting run leaves the posts its compaction covers unread
		let more = more || (stop.is_some() && stop > floor);
		let run_start = more.then(|| run.first().map(Post::id)).flatten();
		posts.append(&mut run);
		let mut records = self.records_of(&lineage, posts, run_start).await?;
		records.set_seed(self.lineage_seed(&lineage).await?);
		records.xok()
	}

	/// The [`Self::load`] posts before the run of `records`, a window of
	/// `thread`, as records to merge under them
	/// ([`ThreadRecords::merge_stored`]): stopping at the seed it holds, and
	/// with no posts and no run start once nothing older is left.
	///
	/// ## Errors
	///
	/// Errors as [`Self::open`].
	pub async fn read_older(
		&self,
		thread: &Thread,
		records: &ThreadRecords,
	) -> Result<ThreadRecords> {
		match records.run_start() {
			Some(start) => {
				self.read_before(thread, start, records.run_floor()).await
			}
			None => Ok(ThreadRecords::new()),
		}
	}

	/// The [`Self::load`] posts of `thread` before `start` and after `floor`.
	async fn read_before(
		&self,
		thread: &Thread,
		start: PostId,
		floor: Option<PostId>,
	) -> Result<ThreadRecords> {
		let lineage = self.lineage(thread).await?;
		let (posts, more) = self
			.posts_back(&lineage, Some(start), floor, Some(self.load))
			.await?;
		let run_start = more.then(|| posts.first().map(Post::id)).flatten();
		self.records_of(&lineage, posts, run_start).await
	}

	/// Read the posts before the run of the window `entity` into its
	/// records ([`Self::read_older`]), answering how many it read: none for a
	/// window holding its whole thread or mounting no store. A window whose
	/// run moved meanwhile keeps its own.
	pub async fn read_older_into(entity: &AsyncEntity) -> Result<usize> {
		let id = entity.id();
		let Some((store, thread, start, floor)) = entity
			.world()
			.with(move |world: &mut World| {
				let store = world.get::<ThreadStore>(id)?.clone();
				let thread = world.get::<Thread>(id)?.clone();
				let records = world.get::<ThreadRecords>(id)?;
				(store, thread, records.run_start()?, records.run_floor())
					.xsome()
			})
			.await
		else {
			return Ok(0);
		};
		let older = store.read_before(&thread, start, floor).await?;
		let read = older.posts().len();
		entity
			.world()
			.with(move |world: &mut World| {
				if let Some(mut records) = world.get_mut::<ThreadRecords>(id)
					&& records.run_start() == Some(start)
				{
					records.merge_stored(older);
				}
			})
			.await;
		read.xok()
	}

	/// Up to `count` posts before `before` along `lineage`, newest first in
	/// the reading and answered in [`PostId`] order, never at or below
	/// `floor`, and whether more are left; every post for `None`.
	async fn posts_back(
		&self,
		lineage: &[(ThreadId, Option<PostId>)],
		before: Option<PostId>,
		floor: Option<PostId>,
		count: Option<usize>,
	) -> Result<(Vec<Post>, bool)> {
		// one past the count says whether more are left
		let wanted = count.map(|count| count + 1);
		let mut found = Vec::new();
		for (id, until) in lineage {
			let remaining = wanted.map(|wanted| wanted - found.len());
			if remaining == Some(0) {
				break;
			}
			// the tighter of the segment's own end and `before`
			let (end, inclusive) = match (before, until) {
				(Some(before), Some(until)) if *until < before => {
					(Some(*until), true)
				}
				(Some(before), _) => (Some(before), false),
				(None, Some(until)) => (Some(*until), true),
				(None, None) => (None, false),
			};
			if floor.zip(end).is_some_and(|(floor, end)| floor >= end) {
				continue;
			}
			let mut scan = Scan::default().reverse();
			scan = match (end, inclusive) {
				(Some(end), true) => scan.through(end),
				(Some(end), false) => scan.before(end),
				(None, _) => scan,
			};
			if let Some(floor) = floor {
				scan = scan.after(floor);
			}
			let table = self.partition(*id)?.table::<Post>();
			let mut rows = match remaining {
				Some(remaining) => table.scan_first(scan, remaining).await?,
				None => table.scan_all(scan).await?,
			};
			for post in &mut rows {
				post.set_thread(*id);
			}
			found.append(&mut rows);
		}
		let more = wanted.is_some_and(|wanted| found.len() >= wanted);
		if let Some(count) = count {
			found.truncate(count);
		}
		found.reverse();
		(found, more).xok()
	}

	/// The seed's last post of `lineage`, its root's ([`Thread::seed`]),
	/// no later than where the branch from the root leaves it.
	async fn lineage_seed(
		&self,
		lineage: &[(ThreadId, Option<PostId>)],
	) -> Result<Option<PostId>> {
		let Some((root, until)) = lineage.last() else {
			return Ok(None);
		};
		let seed = self
			.read_thread(*root)
			.await?
			.and_then(|thread| thread.seed());
		seed.map(|seed| until.map_or(seed, |until| until.min(seed)))
			.xok()
	}

	/// The seed of `lineage`, its root's posts through the root's seed
	/// ([`Thread::seed`]), a branch's being its lineage root's.
	async fn seed_posts(
		&self,
		lineage: &[(ThreadId, Option<PostId>)],
	) -> Result<Vec<Post>> {
		let Some((root, _)) = lineage.last() else {
			return Ok(Vec::new());
		};
		let Some(through) = self.lineage_seed(lineage).await? else {
			return Ok(Vec::new());
		};
		let mut posts = self
			.partition(*root)?
			.table::<Post>()
			.scan_all(Scan::default().through(through))
			.await?;
		for post in &mut posts {
			post.set_thread(*root);
		}
		posts.xok()
	}

	/// Records holding `posts`, read along `lineage`, with the actors that
	/// authored them and their metadata, each read from the partition of
	/// the post over the span loaded from it, and the run starting at
	/// `run_start`.
	async fn records_of(
		&self,
		lineage: &[(ThreadId, Option<PostId>)],
		posts: Vec<Post>,
		run_start: Option<PostId>,
	) -> Result<ThreadRecords> {
		let mut records = ThreadRecords::new();
		for (id, _) in lineage {
			let held = posts
				.iter()
				.filter(|post| post.thread() == *id)
				.collect::<Vec<_>>();
			let (Some(first), Some(last)) = (held.first(), held.last()) else {
				continue;
			};
			let partition = self.partition(*id)?;
			for post in &held {
				if records.actor(post.author()).is_err()
					&& let Some(actor) = partition
						.table::<Actor>()
						.get(post.author().clone())
						.await?
				{
					records.insert_actor(actor);
				}
			}
			let ids = held.iter().map(|post| post.id()).collect::<HashSet<_>>();
			for meta in partition
				.table::<ResponseMeta>()
				.scan_all(Scan::default().from(first.id()).through(last.id()))
				.await?
				.into_iter()
				.filter(|meta| ids.contains(&meta.post_id))
			{
				records.set_meta(meta);
			}
		}
		for post in posts {
			records.upsert_post(post);
		}
		records.set_run_start(run_start);
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
	pub(super) fn partition(&self, thread: ThreadId) -> Result<TableStore> {
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

	/// Write the bytes of each file a post holds in memory to `blobs/<cid>`,
	/// addressed by content so the same bytes twice are one blob. Without a
	/// blob store the bytes stay in memory on the window. Nothing removes a
	/// blob: a post edited or deleted keeps its file, as it keeps its row.
	pub(super) async fn write_blobs(&self, posts: &[Post]) -> Result {
		let Some(blobs) = &self.blobs else {
			return Ok(());
		};
		for post in posts {
			for file in post.files() {
				let Some(bytes) = post.file_bytes(file.index()) else {
					continue;
				};
				let path = Self::blob_path(file.blob());
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

/// A window's flush progress: the revision of its records the flush last
/// read, the window's own posts and metadata not yet written, and each actor
/// as last written, so a flush writes only the window's own new records. It
/// holds what is unwritten, never what is, so it does not grow with the
/// thread; a post loaded or received is never the window's to write.
#[derive(Debug, Default, Clone, Component)]
pub struct SyncedRecords {
	/// The revision of the records the flush last read.
	read: Revision,
	/// The window's own posts not yet written, ie still in progress.
	posts: BTreeSet<PostId>,
	/// The metadata set here not yet written.
	metas: BTreeSet<PostId>,
	/// Each actor as last written, or as loaded.
	actors: HashMap<ActorId, Actor>,
}

impl SyncedRecords {
	/// Every record of `records` held already, ie as loaded.
	pub fn of(records: &ThreadRecords) -> Self {
		let mut synced = Self {
			read: records.revision(),
			..default()
		};
		synced.hold_actors(records.actors());
		synced
	}

	/// Mark `actors` written as they stand, ie as loaded.
	pub fn hold_actors(&mut self, actors: &[Actor]) {
		self.actors.extend(
			actors
				.iter()
				.map(|actor| (actor.id().clone(), actor.clone())),
		);
	}

	/// Mark the actors of `changes`, another window's, written: its author's
	/// window writes them.
	pub fn receive(&mut self, changes: &ThreadChanges) {
		self.hold_actors(&changes.actors);
	}

	/// The window's own posts `records` holds that are not yet written: those
	/// queued, and those changed here since the flush last read.
	pub fn unwritten<'a>(
		&'a self,
		records: &'a ThreadRecords,
	) -> impl Iterator<Item = PostId> + 'a {
		self.posts.iter().copied().chain(
			records
				.changed_since(self.read)
				.filter(|(_, change)| change.local_after(self.read))
				.map(|(id, _)| id),
		)
	}

	/// Queue what `records` changed here since the flush last read.
	fn read(&mut self, records: &ThreadRecords) {
		for (id, change) in records.changed_since(self.read) {
			if change.local_after(self.read) {
				self.posts.insert(id);
			}
			if change.meta_after(self.read) {
				self.metas.insert(id);
			}
		}
		self.read = records.revision();
	}
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
	/// What the records changed here that is not yet written, marking it
	/// written, or `None` when everything is. Reads only what changed since
	/// the last take. A post in progress stays queued until it completes,
	/// and so does its metadata.
	pub(crate) fn take(
		thread: &Thread,
		records: &ThreadRecords,
		synced: &mut SyncedRecords,
	) -> Option<Self> {
		synced.read(records);
		let ready = synced
			.posts
			.iter()
			.copied()
			.filter(|id| {
				records.post(*id).is_none_or(|post| !post.in_progress())
			})
			.collect::<Vec<_>>();
		let mut posts = Vec::new();
		for id in ready {
			synced.posts.remove(&id);
			if let Some(post) =
				records.post(id).filter(|post| post.thread() == thread.id())
			{
				posts.push(post.clone());
			}
		}
		// the metadata of posts written, ie a usage landing after its post
		// flushed, or written now
		let ready = synced
			.metas
			.iter()
			.copied()
			.filter(|id| !synced.posts.contains(id))
			.collect::<Vec<_>>();
		let mut metas = Vec::new();
		for id in ready {
			synced.metas.remove(&id);
			if let Some(meta) = records.meta(id) {
				metas.push(meta.clone());
			}
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
	fn texts(posts: &[impl core::borrow::Borrow<Post>]) -> Vec<String> {
		posts.iter().map(|post| post.borrow().to_string()).collect()
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
		let mut records = store.open(&thread, Reach::Whole).await.unwrap();
		let mut synced = SyncedRecords::of(&records);
		records.set_meta(meta.clone());
		store
			.write(ThreadFlush::take(&thread, &records, &mut synced).unwrap())
			.await
			.unwrap();
		store
			.open(&thread, Reach::Whole)
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
			let records = store.open(&branch, Reach::Whole).await.unwrap();
			texts(records.posts()).xpect_eq(vec!["post 0", "post 1", text]);
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

		// a change puts the post under the cid it was read at with its log,
		// and a second change from the same stale read is refused
		let stored_mine = store
			.posts(thread.id())
			.await
			.unwrap()
			.into_iter()
			.find(|post| post.id() == mine.id())
			.unwrap();
		let change = |text: &str| {
			let mut changed = stored_mine.clone();
			changed.set_text(text);
			let log =
				Post::new_log(actor_id.clone(), thread.id(), &PostEdited {
					was: stored_mine.body().clone(),
				})
				.unwrap()
				.with_subject(mine.id());
			[changed, log]
		};
		store.change(&stored_mine, &change("edited")).await.unwrap();
		store
			.change(&stored_mine, &change("again"))
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("changed since it was read");
		let posts = store.posts(thread.id()).await.unwrap();
		texts(&posts[..6]).xpect_eq(vec![
			"post 0", "post 1", "post 2", "edited", "theirs", "[edited]",
		]);
		posts.len().xpect_eq(6);

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
			post.holds_bytes().xpect_false();
			store
				.read_blob(post.files()[0].blob())
				.await
				.unwrap()
				.to_vec()
				.xpect_eq(vec![1, 2, 3]);
		}
	}

	/// A store of `load` over memory, and `count` text posts by the user.
	fn long_thread(load: usize) -> (ThreadStore, Actor) {
		let store = ThreadStore::new(
			TableStore::new(InMemoryTableStore::new()),
			RelPath::new("threads"),
		)
		.unwrap()
		.with_load(load);
		(store, Actor::user())
	}

	/// `count` text posts by `actor` in `thread`, `post 0` first.
	fn numbered(actor: &Actor, thread: ThreadId, count: usize) -> Vec<Post> {
		(0..count)
			.map(|i| post(actor.id(), thread, &format!("post {i}")))
			.collect()
	}

	/// Read every older run into `records`, answering the ids in the order
	/// they were read, newest first.
	async fn read_back(
		store: &ThreadStore,
		thread: &Thread,
		records: &mut ThreadRecords,
	) -> Vec<PostId> {
		let mut read = records
			.posts()
			.iter()
			.rev()
			.map(|post| post.id())
			.collect::<Vec<_>>();
		while records.has_older() {
			let older = store.read_older(thread, records).await.unwrap();
			read.extend(older.posts().iter().rev().map(|post| post.id()));
			records.merge_stored(older);
		}
		read
	}

	/// A window opens on the newest posts, and reading back to the first
	/// post yields every post once: in order in the records, and in reverse
	/// as it was read.
	#[beet_core::test]
	async fn reads_back_to_the_first_post() {
		let (store, actor) = long_thread(7);
		let thread = Thread::new("Long");
		let posts = numbered(&actor, thread.id(), 50);
		store
			.store_thread(&thread, vec![actor.clone()], posts.clone())
			.await
			.unwrap();
		let mut records = store
			.open(&thread, Reach::Newest {
				count: 7,
				seed: false,
			})
			.await
			.unwrap();
		texts(records.posts()).xpect_eq(texts(&posts[43..]));
		records.has_older().xpect_true();
		records.actor(actor.id()).unwrap();
		let read = read_back(&store, &thread, &mut records).await;
		let ids = posts.iter().map(Post::id).collect::<Vec<_>>();
		records
			.posts()
			.iter()
			.map(|post| post.id())
			.collect::<Vec<_>>()
			.xpect_eq(ids.clone());
		read.into_iter().rev().collect::<Vec<_>>().xpect_eq(ids);
		// the whole thread reads at once, and nothing is older
		let whole = store.open(&thread, Reach::Whole).await.unwrap();
		whole.posts().len().xpect_eq(50);
		whole.has_older().xpect_false();
	}

	/// A window holding the seed opens on it and the newest posts, and
	/// reading back stops at the seed, every post held once.
	#[beet_core::test]
	async fn the_seed_is_held_apart_from_the_run() {
		let (store, actor) = long_thread(5);
		let mut thread = Thread::new("Seeded");
		let posts = numbered(&actor, thread.id(), 30);
		thread.set_seed(Some(posts[1].id()));
		store
			.store_thread(&thread, vec![actor.clone()], posts.clone())
			.await
			.unwrap();
		let mut records = store
			.open(&thread, Reach::Newest {
				count: 5,
				seed: true,
			})
			.await
			.unwrap();
		texts(records.posts()).xpect_eq(
			[
				"post 0", "post 1", "post 25", "post 26", "post 27", "post 28",
				"post 29",
			]
			.map(String::from)
			.to_vec(),
		);
		records.run_floor().xpect_eq(Some(posts[1].id()));
		read_back(&store, &thread, &mut records).await;
		texts(records.posts()).xpect_eq(texts(&posts));
	}

	/// A branch opens on a run crossing into its parent, and reads back
	/// through the parent's posts up to the branch point.
	#[beet_core::test]
	async fn a_branchs_run_crosses_into_its_parent() {
		let (store, actor) = long_thread(4);
		let parent = Thread::new("Parent");
		let parent_posts = numbered(&actor, parent.id(), 20);
		store
			.store_thread(&parent, vec![actor.clone()], parent_posts.clone())
			.await
			.unwrap();
		let branch = Thread::new("Branch")
			.with_branch_of(PostRef::new(parent.id(), parent_posts[9].id()));
		let own = vec![
			post(actor.id(), branch.id(), "mine 0"),
			post(actor.id(), branch.id(), "mine 1"),
		];
		store
			.store_thread(&branch, vec![actor.clone()], own.clone())
			.await
			.unwrap();
		let mut records = store
			.open(&branch, Reach::Newest {
				count: 4,
				seed: false,
			})
			.await
			.unwrap();
		texts(records.posts()).xpect_eq(
			["post 8", "post 9", "mine 0", "mine 1"]
				.map(String::from)
				.to_vec(),
		);
		records.posts()[0].thread().xpect_eq(parent.id());
		read_back(&store, &branch, &mut records).await;
		let mut expected = texts(&parent_posts[..10]);
		expected.extend(texts(&own));
		texts(records.posts()).xpect_eq(expected);
	}

	/// A run reaches back to the last word, past any logs newer than it.
	#[beet_core::test]
	async fn a_run_holds_the_last_word() {
		let (store, actor) = long_thread(3);
		let thread = Thread::new("Joins");
		let mut posts = vec![post(actor.id(), thread.id(), "said")];
		for _ in 0..8 {
			posts.push(
				Post::new_log(
					actor.id().clone(),
					thread.id(),
					&SessionOpened::default(),
				)
				.unwrap(),
			);
		}
		store
			.store_thread(&thread, vec![actor.clone()], posts)
			.await
			.unwrap();
		let records = store
			.open(&thread, Reach::Newest {
				count: 3,
				seed: false,
			})
			.await
			.unwrap();
		records.last_word().unwrap().to_string().xpect_eq("said");
		records.has_older().xpect_false();
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
