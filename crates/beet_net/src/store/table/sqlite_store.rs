use crate::prelude::*;
use beet_core::prelude::*;
use rusqlite::Connection;
use rusqlite::types::Value as SqlValue;
use rusqlite::types::ValueRef;
use std::sync::Arc;
use std::sync::Mutex;

/// A SQLite database file as a table store: each [`TableStoreRow`] table a real
/// SQL table (`key TEXT PRIMARY KEY, json TEXT NOT NULL, cid TEXT NOT NULL`, see
/// its [`TableProvider`] impl), so a backend-agnostic caller reaches it through
/// the erased [`TableStore`] like any other while a consumer that chose SQLite
/// indexes and queries the rows with SQL ([`execute`](Self::execute),
/// [`query`](Self::query)), or from markup through
/// [`SqlSelect`](crate::prelude::SqlSelect). Rows alone: blobs in the same file
/// are a [`SqliteBlobStore`]'s.
///
/// One pooled connection per path is shared by every clone of the store, opened
/// in WAL mode on first use (which creates the file); every operation runs on
/// the blocking pool, never on the async executor. Rows are stored as canonical json (sorted keys), so the same document always stores
/// the same text and `json = ?` comparisons are meaningful.
#[derive(Debug, Clone, Component, Get, Reflect)]
#[reflect(Component, Default)]
#[component(on_insert = TableStore::on_insert::<Self>)]
pub struct SqliteStore {
	/// The database file. Coerces from a workspace-relative string attribute
	/// in markup, ie `<SqliteStore path="data.db"/>`.
	path: AbsPath,
}

/// The pooled connection, a [`Mutex`] since a [`Connection`] is not [`Sync`].
type SharedConnection = Arc<Mutex<Connection>>;

cfg_if! {
	if #[cfg(target_arch = "wasm32")] {
		/// What a future must be to reach [`SqliteStore::erase`]: nothing in
		/// particular on wasm, where there is one thread and `SendWrapper`
		/// asserts the rest.
		trait Erasable: 'static + Future {}
		impl<T: 'static + Future> Erasable for T {}
	} else {
		/// What a future must be to reach [`SqliteStore::erase`]: [`Send`],
		/// since the executor it lands on is threaded.
		trait Erasable: 'static + Send + Future {}
		impl<T: 'static + Send + Future> Erasable for T {}
	}
}

/// One connection per database file, shared by every store on that path.
static POOL: LazyPool<AbsPath, SharedConnection, Result<SharedConnection>> =
	LazyPool::new(|path| {
		let path = path.clone();
		Box::pin(async move {
			SqliteStore::offload(move || SqliteStore::open(&path)).await
		})
	});

/// The workspace's own `data.db`, so `<SqliteStore/>` names the repo's index
/// and `<SqliteStore path="other.db"/>` any other database. A default is what
/// makes the tag authorable at all: markup builds the default and writes the
/// attributes over it.
impl Default for SqliteStore {
	fn default() -> Self { Self::new(WsPath::new("data.db")) }
}

impl SqliteStore {
	/// The path prefix naming a database held in memory rather than a file.
	const MEMORY_ROOT: &'static str = "/:memory:";

	/// A database held in memory under `name`, shared by every store of that
	/// name in this process and gone when the last is removed: what an
	/// archive is built in before it is serialized, and where a fetched one
	/// is opened.
	pub fn memory(name: impl AsRef<str>) -> Self {
		Self::new(AbsPath::new_unchecked(format!(
			"{}/{}",
			Self::MEMORY_ROOT,
			name.as_ref()
		)))
	}

	/// A read-only database opened in memory from the `bytes` of a database
	/// file, ie an archive fetched from a bucket, under `name`: queryable with
	/// [`SqlSelect`](crate::prelude::SqlSelect), [`query`](Self::query) and
	/// the same [`Table`] reads as any database, refusing every write.
	pub async fn from_bytes(
		name: impl AsRef<str>,
		bytes: Vec<u8>,
	) -> Result<Self> {
		let store = Self::memory(name);
		let conn = Self::offload(move || {
			let mut conn = Connection::open_in_memory()?;
			conn.deserialize_read_exact(
				rusqlite::MAIN_DB,
				std::io::Cursor::new(&bytes),
				bytes.len(),
				true,
			)?;
			conn.pragma_update(None, "query_only", true)?;
			Arc::new(Mutex::new(conn)).xok()
		})
		.await?;
		POOL.insert(store.path.clone(), conn).await;
		store.xok()
	}

	/// The whole database as the bytes of its file, what an archive writes
	/// to a bucket: serialized from the live connection, so it works alike
	/// for a file and a database held in memory, a Worker's included.
	pub async fn to_bytes(&self) -> Result<Vec<u8>> {
		self.run(|conn| conn.serialize(rusqlite::MAIN_DB)?.to_vec().xok())
			.await
	}

	/// Whether this database is held in memory rather than in a file: every
	/// database on wasm, whose default VFS is memory, and a
	/// [`memory`](Self::memory) one anywhere.
	fn in_memory(&self) -> bool {
		cfg!(target_arch = "wasm32")
			|| self.path.as_str().starts_with(Self::MEMORY_ROOT)
	}

	/// The store at the database file `path`, created on first use.
	pub fn new(path: impl Into<AbsPath>) -> Self { Self { path: path.into() } }

	/// The database file of partition `id`: `<dir>/<stem>/<id>.db` beside
	/// this one, a nested id a nested directory, so a partition is one
	/// exportable file and [`SqlSelect`](crate::prelude::SqlSelect),
	/// [`execute`](Self::execute) and [`query`](Self::query) work against it
	/// as against any database.
	pub fn partition_store(&self, id: &PartitionId) -> Self {
		let mut path = self.partitions_dir();
		let segments = id.segments().collect::<Vec<_>>();
		for (idx, segment) in segments.iter().enumerate() {
			path = match idx + 1 == segments.len() {
				true => path.join(format!("{segment}.db")),
				false => path.join(segment),
			};
		}
		Self::new(path)
	}

	/// The directory this database's partitions live in, `<dir>/<stem>`.
	fn partitions_dir(&self) -> AbsPath {
		let stem = self
			.path
			.file_stem()
			.map(|stem| stem.to_string())
			.unwrap_or_default();
		self.path
			.parent()
			.map(|parent| parent.join(&stem))
			.unwrap_or_else(|| AbsPath::new_unchecked(stem))
	}

	/// The store a `sqlite:<path>` uri names, a relative path resolved against
	/// the cwd, see [`StoreUri::Sqlite`].
	pub fn from_uri(uri: &StoreUri) -> Result<Self> {
		let StoreUri::Sqlite { path } = uri else {
			bevybail!("store `{uri}` is not a sqlite: database");
		};
		Self::new(AbsPath::new(path)?).xok()
	}

	/// Run a statement, or a `;` separated batch, returning the rows changed
	/// by the last: how a consumer creates views, indexes over
	/// `json_extract(json, '$.field')` and FTS5 tables. Deliberately not on
	/// [`TableProvider`]: SQL is this backend's own, and a caller reaching for
	/// it has already chosen SQLite.
	pub async fn execute(&self, sql: &str) -> Result<usize> {
		let sql = sql.to_string();
		self.run(move |conn| {
			conn.execute_batch(&sql)?;
			conn.changes().xmap(|changed| changed as usize).xok()
		})
		.await
	}

	/// Run a query, each row a [`Value::Map`] keyed by column name. The SQL
	/// twin of [`execute`](Self::execute), see there.
	pub async fn query(&self, sql: &str) -> Result<Vec<Value>> {
		let sql = sql.to_string();
		self.run(move |conn| {
			let mut stmt = conn.prepare(&sql)?;
			let columns = stmt
				.column_names()
				.into_iter()
				.map(SmolStr::from)
				.collect::<Vec<_>>();
			let mut rows = stmt.query([])?;
			let mut out = Vec::new();
			while let Some(row) = rows.next()? {
				columns
					.iter()
					.enumerate()
					.map(|(idx, column)| {
						Ok((column.clone(), value_of(row.get_ref(idx)?)?))
					})
					.collect::<Result<Map>>()?
					.xmap(Value::Map)
					.xmap(|value| out.push(value));
			}
			out.xok()
		})
		.await
	}

	/// Get or open the pooled connection for this store's path.
	async fn connection(&self) -> Result<SharedConnection> {
		POOL.try_get(&self.path).await
	}

	/// Evict this path's pooled connection, closing it, so the next operation
	/// opens afresh: what [`store_remove`](StoreBackend::store_remove)
	/// does before deleting the file the connection was open on.
	async fn close(&self) {
		let conn = POOL.remove(&self.path).await;
		// closing checkpoints the WAL, filesystem work to offload
		let _ = Self::offload(move || Ok(drop(conn))).await;
	}

	/// Open the database at `path`, creating it. WAL
	/// journaling with normal sync: durable across a process crash, and a
	/// reader never blocks the writer.
	///
	/// Both of those are about a real file, so neither happens on wasm, whose
	/// default VFS holds the database in memory: `path` is a name there rather
	/// than a location, there is no directory to create, no second process to
	/// shield and no shared memory to journal through. Registering a durable
	/// browser VFS (OPFS) is the seam to widen this at.
	///
	/// A [`memory`](Self::memory) path opens a database in memory anywhere.
	fn open(path: &AbsPath) -> Result<SharedConnection> {
		let conn = match path.as_str().starts_with(Self::MEMORY_ROOT) {
			true => Connection::open_in_memory()?,
			false => {
				#[cfg(not(target_arch = "wasm32"))]
				if let Some(parent) = path.parent() {
					fs_ext::create_dir_all(&parent)?;
				}
				let conn = Connection::open(path.as_str())?;
				#[cfg(not(target_arch = "wasm32"))]
				{
					conn.pragma_update(None, "journal_mode", "WAL")?;
					conn.pragma_update(None, "synchronous", "NORMAL")?;
				}
				conn
			}
		};
		Arc::new(Mutex::new(conn)).xok()
	}

	/// Run `func` against the pooled connection, off the async executor.
	pub(crate) fn run<T: 'static + Send>(
		&self,
		func: impl 'static + Send + FnOnce(&Connection) -> Result<T>,
	) -> SendBoxedFuture<Result<T>> {
		let this = self.clone();
		Self::erase(async move {
			let conn = this.connection().await?;
			Self::offload(move || {
				// a poisoned lock only means a panic mid-operation; the
				// connection itself is still consistent
				let conn = conn.lock().unwrap_or_else(|err| err.into_inner());
				func(&conn)
			})
			.await
		})
	}

	/// Erase a pooled operation into the currency every store provider answers
	/// in, which is [`Send`].
	///
	/// The pool's own future is not, on wasm: bevy's `BoxedFuture` is its
	/// conditional-send alias there. With one thread to be on, wrapping is the
	/// same assertion every browser store in this module already makes.
	fn erase<T: 'static>(fut: impl Erasable<Output = T>) -> SendBoxedFuture<T> {
		cfg_if! {
			if #[cfg(target_arch = "wasm32")] {
				Box::pin(send_wrapper::SendWrapper::new(fut))
			} else {
				Box::pin(fut)
			}
		}
	}

	/// Run blocking SQLite work off the async executor, which natively means
	/// the `blocking` pool.
	///
	/// On wasm it means HERE: there are no threads to hand it to, so the call
	/// occupies whichever one it is on for its duration. That is the whole of
	/// the seam, and it is internal — every caller, native included, sees the
	/// same future. A browser build that cares about jank belongs in a
	/// dedicated worker for the same reason its durable VFS would.
	fn offload<T: 'static + Send>(
		func: impl 'static + Send + FnOnce() -> Result<T>,
	) -> SendBoxedFuture<Result<T>> {
		cfg_if! {
			if #[cfg(target_arch = "wasm32")] {
				Box::pin(core::future::ready(func()))
			} else {
				Box::pin(blocking::unblock(func))
			}
		}
	}
}

/// A column value as a [`Value`]: SQLite's five storage classes map one to
/// one, text required to be utf8.
fn value_of(value: ValueRef) -> Result<Value> {
	match value {
		ValueRef::Null => Value::Null,
		ValueRef::Integer(int) => Value::Int(int),
		ValueRef::Real(float) => Value::Float(float),
		ValueRef::Text(text) => Value::Str(core::str::from_utf8(text)?.into()),
		ValueRef::Blob(blob) => Value::Bytes(blob.to_vec()),
	}
	.xok()
}

impl StoreBackend for SqliteStore {
	fn id(&self) -> &'static str { "sqlite" }

	fn describe(&self) -> String { format!("sqlite:{}", self.path) }

	fn region(&self) -> Option<String> { None }

	/// Whether the database is there to open.
	///
	/// Natively, and under any file-backed VFS, that is a file on disk. The
	/// default wasm VFS holds the database in memory, where `path` is a name
	/// rather than a location, so it exists exactly while this process holds a
	/// connection to it.
	fn store_exists(&self) -> SendBoxedFuture<Result<bool>> {
		let path = self.path.clone();
		match self.in_memory() {
			true => {
				Self::erase(async move { POOL.contains(&path).await.xok() })
			}
			false => {
				Box::pin(async move { fs_ext::exists_async(path).await?.xok() })
			}
		}
	}

	/// Create the database file.
	///
	/// # Errors
	/// Fails if the file already exists.
	fn store_create(&self) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Self::erase(async move {
			if this.store_exists().await? {
				bevybail!("sqlite store `{}` already exists", this.path);
			}
			this.connection().await.map(drop)
		})
	}

	/// Close the pooled connection and delete the file, a
	/// [`SqliteBlobStore`]'s blobs in it included, with the WAL sidecars a
	/// crash may have left beside it and every partition's file. Under the wasm memory VFS closing IS
	/// the delete: the database lives in the connection, and there is no file
	/// and no sidecar to unlink.
	///
	/// # Errors
	/// Fails if the database does not exist.
	fn store_remove(&self) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Self::erase(async move {
			if !this.store_exists().await? {
				bevybail!("sqlite store `{}` does not exist", this.path);
			}
			this.close().await;
			// this database's partitions go with it
			let partitions = this.partitions_dir();
			let closed = POOL
				.remove_where(|path| {
					path.as_str().starts_with(&format!("{partitions}/"))
				})
				.await;
			let _ = Self::offload(move || Ok(drop(closed))).await;
			if !this.in_memory() {
				if fs_ext::exists_async(&partitions).await? {
					fs_ext::remove_async(&partitions).await?;
				}
				fs_ext::remove_async(&this.path).await?;
				for sidecar in ["-wal", "-shm"] {
					let sidecar = format!("{}{sidecar}", this.path);
					if fs_ext::exists_async(&sidecar).await? {
						fs_ext::remove_async(&sidecar).await?;
					}
				}
			}
			Ok(())
		})
	}
}

/// Native table form: [`SqlTable`]'s schema and semantics run through this
/// connection, every precondition one statement under the connection's lock
/// and [`apply`](TableProvider::apply) one transaction. A partition is its own
/// database file, `<dir>/<stem>/<partition>.db` beside this one
/// ([`partition_store`](Self::partition_store)), so one partition is one
/// exportable file.
impl TableProvider for SqliteStore {
	fn box_clone_table(&self) -> Box<dyn TableProvider> {
		Box::new(self.clone())
	}

	fn partition(&self, id: &PartitionId) -> Box<dyn TableProvider> {
		Box::new(self.partition_store(id))
	}

	fn supports(&self, _capability: Capability) -> bool { true }

	fn get_row(
		&self,
		table: &TableSpec,
		key: &TableKey,
	) -> SendBoxedFuture<Result<Option<StoredRow>>> {
		let table = table.clone();
		let key = key.clone();
		self.run(move |conn| SqlTable::new(&table)?.get(conn, &key))
	}

	fn put_row(
		&self,
		table: &TableSpec,
		row: StoredRow,
		precondition: Precondition,
	) -> SendBoxedFuture<Result> {
		let table = table.clone();
		self.run(move |conn| {
			SqlTable::new(&table)?.put(conn, &row, &precondition)
		})
	}

	fn remove_row(
		&self,
		table: &TableSpec,
		key: &TableKey,
		precondition: Precondition,
	) -> SendBoxedFuture<Result> {
		let table = table.clone();
		let key = key.clone();
		self.run(move |conn| {
			SqlTable::new(&table)?.remove(conn, &key, &precondition)
		})
	}

	fn scan_rows(
		&self,
		table: &TableSpec,
		scan: Scan,
	) -> SendBoxedFuture<Result<ScanPage>> {
		let table = table.clone();
		self.run(move |conn| SqlTable::new(&table)?.scan(conn, &scan))
	}

	fn apply(&self, writes: Vec<TableWrite>) -> SendBoxedFuture<Result> {
		self.run(move |conn| SqlTable::apply(conn, &writes))
	}

	fn scan_index(
		&self,
		table: &TableSpec,
		index: &TableIndex,
		scan: IndexScan,
	) -> SendBoxedFuture<Result<ScanPage>> {
		let table = table.clone();
		let index = index.clone();
		self.run(move |conn| {
			SqlTable::new(&table)?.scan_index(conn, &index, &scan)
		})
	}
}

/// A rusqlite connection runs [`SqlTable`]'s statements.
impl SqlEngine for Connection {
	fn execute(&self, sql: &str, args: &[SqlArg]) -> Result<usize> {
		Connection::execute(
			self,
			sql,
			rusqlite::params_from_iter(args.iter().map(SqliteStore::to_sql)),
		)?
		.xok()
	}

	fn query(&self, sql: &str, args: &[SqlArg]) -> Result<Vec<Vec<SqlArg>>> {
		let mut stmt = self.prepare(sql)?;
		let columns = stmt.column_count();
		let mut rows = stmt.query(rusqlite::params_from_iter(
			args.iter().map(SqliteStore::to_sql),
		))?;
		let mut out = Vec::new();
		while let Some(row) = rows.next()? {
			out.push(
				(0..columns)
					.map(|idx| SqliteStore::from_sql(row.get_ref(idx)?))
					.collect::<Result<Vec<_>>>()?,
			);
		}
		out.xok()
	}

	fn transaction(
		&self,
		func: &mut dyn FnMut(&dyn SqlEngine) -> Result,
	) -> Result {
		// dropped uncommitted on an error, rolling every statement back
		let tx = self.unchecked_transaction()?;
		func(&*tx)?;
		tx.commit()?;
		Ok(())
	}
}

impl SqliteStore {
	/// An [`SqlArg`] as rusqlite binds it.
	fn to_sql(arg: &SqlArg) -> SqlValue {
		match arg {
			SqlArg::Null => SqlValue::Null,
			SqlArg::Integer(int) => SqlValue::Integer(*int),
			SqlArg::Real(float) => SqlValue::Real(*float),
			SqlArg::Text(text) => SqlValue::Text(text.clone()),
			SqlArg::Blob(blob) => SqlValue::Blob(blob.clone()),
		}
	}

	/// A column as an [`SqlArg`].
	fn from_sql(value: ValueRef) -> Result<SqlArg> {
		match value {
			ValueRef::Null => SqlArg::Null,
			ValueRef::Integer(int) => SqlArg::Integer(int),
			ValueRef::Real(float) => SqlArg::Real(float),
			ValueRef::Text(text) => {
				SqlArg::Text(core::str::from_utf8(text)?.into())
			}
			ValueRef::Blob(blob) => SqlArg::Blob(blob.to_vec()),
		}
		.xok()
	}
}

/// The one port hook the `printf` vendored in `sqlite-wasm-rs`' libc shim
/// leaves to whoever links it, and the only free fn in this module because an
/// `extern "C"` symbol cannot be an associated item.
///
/// `wasm32-unknown-unknown` has no libc, so each C library in a build brings
/// its own shim. SQLite's is the one that defines the standard `printf` names,
/// and its `printf.o` joins the link the moment ANY C code in the binary
/// references one of them — quickjs does, so `sqlite` + `quickjs` in one wasm
/// binary is the case that needs this. That object calls `putchar_`, which the
/// library documents as the caller's to provide, and without it the link fails
/// on an undefined symbol.
///
/// Buffered to a line, since C prints a character at a time and a console logs
/// a call at a time.
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn putchar_(char: core::ffi::c_char) {
	use std::cell::RefCell;
	thread_local! {
		static LINE: RefCell<String> = const { RefCell::new(String::new()) };
	}
	LINE.with_borrow_mut(|line| match char as u8 {
		b'\n' => beet_core::cross_log!("{}", core::mem::take(line)),
		byte => line.push(byte as char),
	});
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A store nothing else shares, and whatever has to outlive it.
	///
	/// Natively that is a real database file under a temp dir, kept alive by
	/// the guard. On wasm the default VFS holds the database in memory, so
	/// there is nothing to keep and the name alone makes it distinct — the
	/// pool is keyed by path, so two tests sharing one would share a
	/// connection.
	fn temp_store() -> (impl Sized, SqliteStore) {
		cfg_if! {
			if #[cfg(target_arch = "wasm32")] {
				use std::sync::atomic::AtomicUsize;
				use std::sync::atomic::Ordering;
				static NEXT: AtomicUsize = AtomicUsize::new(0);
				let nth = NEXT.fetch_add(1, Ordering::SeqCst);
				let path = AbsPath::new_unchecked(format!("/store-{nth}.db"));
				((), SqliteStore::new(path))
			} else {
				let dir = TempDir::new_ws().unwrap();
				let store = SqliteStore::new(dir.join("store.db"));
				(dir, store)
			}
		}
	}

	/// The whole suite, a few hundred commits, each a blocking pool hop.
	#[beet_core::test(timeout_ms = 60_000)]
	async fn table() {
		let (_dir, store) = temp_store();
		table_test::run(TableStore::new(store)).await;
	}

	/// The component lands its table store and no blob store.
	#[beet_core::test]
	fn materializes_tables_alone() {
		let (_dir, store) = temp_store();
		let mut world = World::new();
		let entity = world.spawn(store).id();
		world.flush();
		world.entity(entity).contains::<TableStore>().xpect_true();
		world.entity(entity).contains::<BlobStore>().xpect_false();
	}

	/// The shape a consumer indexes a json table by: `VIRTUAL` columns
	/// generated from the row's `json` plus one index across them, which is
	/// what turns a fold over the rows into a b-tree walk. Worth its own case
	/// because it is the one thing a build swapping SQLite's ffi could plausibly
	/// lose, and the query plan is the only thing that says whether it did.
	#[beet_core::test]
	async fn generated_columns_are_indexed() {
		let (_dir, store) = temp_store();
		let table = Table::<TableItem<u32>>::new(store.clone());
		table.put(TableItem::new(7u32)).await.unwrap();
		store
			.execute(
				"ALTER TABLE table_item ADD COLUMN data INTEGER \
				 GENERATED ALWAYS AS (json_extract(json, '$.data')) VIRTUAL; \
				 CREATE INDEX idx_table_item_data ON table_item (data)",
			)
			.await
			.unwrap();
		store
			.query(
				"EXPLAIN QUERY PLAN SELECT count(*) FROM table_item \
				 WHERE data = 7",
			)
			.await
			.unwrap()
			.xmap(|rows| format!("{rows:?}"))
			.xpect_contains("idx_table_item_data");
	}

	/// A declared index is an expression index the planner walks for an
	/// index scan, rather than sorting the table.
	#[beet_core::test]
	async fn declared_index_is_walked() {
		let (_dir, store) = temp_store();
		let table = Table::<table_test::IndexedRow>::new(store.clone());
		table
			.put(table_test::IndexedRow {
				key: "k".into(),
				group: Value::from("g"),
				rank: 1,
			})
			.await
			.unwrap();
		store
			.query(
				"EXPLAIN QUERY PLAN SELECT key FROM indexed_row \
				 ORDER BY json_extract(json, '$.\"rank\"'), key",
			)
			.await
			.unwrap()
			.xmap(|rows| format!("{rows:?}"))
			.xpect_contains("indexed_row__by_rank")
			.xnot()
			.xpect_contains("TEMP B-TREE");
	}

	/// The escape hatch: a view over `json_extract` reads the rows a typed
	/// table wrote, each result row keyed by column name.
	#[beet_core::test]
	async fn query_reads_json_extract() {
		let (_dir, store) = temp_store();
		let table = Table::<TableItem<u32>>::new(store.clone());
		table.put(TableItem::new(7u32)).await.unwrap();
		table.put(TableItem::new(9u32)).await.unwrap();
		store
			.execute(
				"CREATE VIEW payloads AS \
				 SELECT key, json_extract(json, '$.data') AS data \
				 FROM table_item",
			)
			.await
			.unwrap();
		store
			.query("SELECT data FROM payloads ORDER BY data")
			.await
			.unwrap()
			.xpect_eq(vec![
				Value::Map(Map::new([("data", 7i64)])),
				Value::Map(Map::new([("data", 9i64)])),
			]);
	}

	/// Two writes of the same document with different map insertion order
	/// store identical text and cid, so a diff by text means a diff by
	/// content.
	#[beet_core::test]
	async fn canonical_json_is_stable() {
		let (_dir, store) = temp_store();
		let tables = TableStore::new(store.clone());
		let stored = async |row: Value| {
			tables
				.put(
					"rows",
					StoredRow::new("row", Row::from_value(row).unwrap())
						.unwrap(),
					Precondition::Any,
				)
				.await
				.unwrap();
			store
				.query("SELECT json, cid FROM rows")
				.await
				.unwrap()
				.xmap(|rows| rows[0].clone())
		};
		let text = stored(value!({ "b": 2, "a": 1 })).await;
		text.as_map()
			.unwrap()
			.get("json")
			.unwrap()
			.clone()
			.xpect_eq(Value::from(r#"{"a":1,"b":2}"#));
		stored(value!({ "a": 1, "b": 2 })).await.xpect_eq(text);
	}

	/// An invalid or reserved table name never reaches the SQL, and a record
	/// table's NSID is a valid one.
	#[beet_core::test]
	async fn table_names() {
		let (_dir, store) = temp_store();
		let tables = TableStore::new(store);
		let row = || StoredRow::new("k", Row::default()).unwrap();
		for bad in ["Foo-Bar", "1abc", "", "blobs", "a\"b"] {
			tables.put(bad, row(), Precondition::Any).await.unwrap_err();
		}
		tables
			.get("com.example.note", "k")
			.await
			.unwrap()
			.xpect_none();
	}

	/// A partition is its own database file beside the declared one, which
	/// the SQL escape hatch reads, and removing the store removes it.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	async fn partition_is_a_file() {
		let (_dir, store) = temp_store();
		store.store_try_create().await.unwrap();
		let partition =
			store.partition_store(&PartitionId::parse("did:plc:abc").unwrap());
		partition.path().to_string().xpect_eq(
			store
				.path()
				.parent()
				.unwrap()
				.join("store/did:plc:abc.db")
				.to_string(),
		);
		Table::<table_test::NamedRow>::new(partition.clone())
			.put(table_test::NamedRow::new("x", 1))
			.await
			.unwrap();
		partition
			.query("SELECT key FROM named_rows")
			.await
			.unwrap()
			.len()
			.xpect_eq(1);
		fs_ext::exists(partition.path()).unwrap().xpect_true();
		store.store_remove().await.unwrap();
		fs_ext::exists(partition.path()).unwrap().xpect_false();
	}
}
