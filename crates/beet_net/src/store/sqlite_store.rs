use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::params;
use rusqlite::types::ValueRef;
use std::sync::Arc;
use std::sync::Mutex;

/// A SQLite database file as a store, the one a `sqlite:<path>` uri names:
/// blobs in one `blobs` table, and each [`TableStoreRow`] table as a real SQL
/// table (`key TEXT PRIMARY KEY, json TEXT NOT NULL`), so a backend-agnostic
/// caller reaches it through the erased [`BlobStore`] and [`TableStore`] like
/// any other store while a consumer that chose SQLite indexes and queries the
/// rows with SQL ([`execute`](Self::execute), [`query`](Self::query)).
///
/// One pooled connection per path is shared by every clone and scope of the
/// store, opened in WAL mode on first use (which creates the file); every
/// operation runs on the blocking pool, never on the async executor. Rows are
/// stored as canonical json (sorted keys), so the same document always stores
/// the same text and `json = ?` comparisons are meaningful.
#[derive(Debug, Clone, Component, Get, Reflect)]
#[reflect(Component)]
#[component(on_insert = on_insert_sqlite)]
pub struct SqliteStore {
	/// The database file. Coerces from a workspace-relative string attribute
	/// in markup, ie `<SqliteStore path="data.db"/>`.
	path: AbsPath,
	/// Optional key prefix for blobs. Rows are store-wide: a SQL table is a
	/// namespace of its own and takes no prefix.
	#[get(skip)]
	#[reflect(default)]
	subdir: Option<RelPath>,
}

/// The pooled connection, a [`Mutex`] since a [`Connection`] is not [`Sync`].
type SharedConnection = Arc<Mutex<Connection>>;

/// One connection per database file, shared by every store on that path.
static POOL: LazyPool<AbsPath, SharedConnection, Result<SharedConnection>> =
	LazyPool::new(|path| {
		let path = path.clone();
		Box::pin(async move {
			blocking::unblock(move || SqliteStore::open(&path)).await
		})
	});

impl SqliteStore {
	/// The store at the database file `path`, created on first use.
	pub fn new(path: impl Into<AbsPath>) -> Self {
		Self {
			path: path.into(),
			subdir: None,
		}
	}

	/// Set the key prefix for all blobs.
	pub fn with_subdir(mut self, subdir: impl Into<RelPath>) -> Self {
		self.subdir = Some(subdir.into());
		self
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
	/// opens afresh: what [`store_remove`](BlobStoreProvider::store_remove)
	/// does before deleting the file the connection was open on.
	async fn close(&self) {
		let conn = POOL.remove(&self.path).await;
		// closing checkpoints the WAL, filesystem work for the blocking pool
		blocking::unblock(move || drop(conn)).await;
	}

	/// Open the database at `path`, creating it and the `blobs` table. WAL
	/// journaling with normal sync: durable across a process crash, and a
	/// reader never blocks the writer.
	fn open(path: &AbsPath) -> Result<SharedConnection> {
		if let Some(parent) = path.parent() {
			fs_ext::create_dir_all(&parent)?;
		}
		let conn = Connection::open(path.as_str())?;
		conn.pragma_update(None, "journal_mode", "WAL")?;
		conn.pragma_update(None, "synchronous", "NORMAL")?;
		conn.execute_batch(
			"CREATE TABLE IF NOT EXISTS blobs (\
				path TEXT PRIMARY KEY, body BLOB NOT NULL)",
		)?;
		Arc::new(Mutex::new(conn)).xok()
	}

	/// Run `func` against the pooled connection on the blocking pool.
	fn run<T: 'static + Send>(
		&self,
		func: impl 'static + Send + FnOnce(&Connection) -> Result<T>,
	) -> SendBoxedFuture<Result<T>> {
		let this = self.clone();
		Box::pin(async move {
			let conn = this.connection().await?;
			blocking::unblock(move || {
				// a poisoned lock only means a panic mid-operation; the
				// connection itself is still consistent
				let conn = conn.lock().unwrap_or_else(|err| err.into_inner());
				func(&conn)
			})
			.await
		})
	}

	/// The `path` column `path` keys to: the path under this store's subdir.
	fn qualify(&self, path: &RelPath) -> String {
		match &self.subdir {
			Some(sub) => format!("{sub}/{path}"),
			None => path.to_string(),
		}
	}

	/// The `path` prefix every blob in this scope shares, empty at the root.
	fn prefix(&self) -> String {
		self.subdir
			.as_ref()
			.map(|sub| format!("{sub}/"))
			.unwrap_or_default()
	}

	/// A 404 for a missing blob or row, matching every other backend so a
	/// served route distinguishes an absent object from a broken store.
	fn not_found(what: impl core::fmt::Display) -> BevyError {
		HttpError::new(StatusCode::NOT_FOUND, format!("{what} not found"))
			.into()
	}
}

/// Insert both erased store currencies: the [`BlobStore`] every provider gets,
/// then the [`TableStore`] wrapping this provider directly, so its SQL tables
/// win over the json-over-blobs table the blob hook materializes.
fn on_insert_sqlite(mut world: DeferredWorld, cx: HookContext) {
	BlobStore::on_insert::<SqliteStore>(world.reborrow(), cx);
	TableStore::on_insert::<SqliteStore>(world, cx);
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

/// The quoted identifier for `table`, validated against `[a-z][a-z0-9_]*`
/// (the shape [`TableStoreRow::table_name`] produces) so a name never reaches
/// the SQL text unchecked, and refusing `blobs`, the blob table.
fn table_ident(table: &str) -> Result<String> {
	let valid = table
		.chars()
		.next()
		.is_some_and(|first| first.is_ascii_lowercase())
		&& table.chars().all(|ch| {
			ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_'
		});
	if !valid {
		bevybail!(
			"invalid sqlite table name `{table}`: expected [a-z][a-z0-9_]*"
		);
	}
	if table == "blobs" {
		bevybail!("`blobs` is the sqlite blob table, not a row table");
	}
	format!("\"{table}\"").xok()
}

/// Whether the row table `table` exists, for a read that must answer "none"
/// rather than create it.
fn table_exists(conn: &Connection, table: &str) -> Result<bool> {
	conn.query_row(
		"SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
		params![table],
		|_| Ok(()),
	)
	.optional()?
	.is_some()
	.xok()
}

/// Create the row table `ident` if it is missing, for a write.
fn ensure_table(conn: &Connection, ident: &str) -> Result {
	conn.execute_batch(&format!(
		"CREATE TABLE IF NOT EXISTS {ident} (\
			key TEXT PRIMARY KEY, json TEXT NOT NULL)"
	))?;
	Ok(())
}

/// The canonical text of a row: sorted keys via `serde_json::Value`, never
/// beet's insertion-ordered [`Value::Map`].
fn canonical_json(row: Value) -> Result<String> {
	serde_json::to_string(&serde_json::Value::from(row))?.xok()
}

impl BlobStoreProvider for SqliteStore {
	fn box_clone(&self) -> Box<dyn BlobStoreProvider> { Box::new(self.clone()) }

	fn with_subdir(&self, path: RelPath) -> Box<dyn BlobStoreProvider> {
		Box::new(SqliteStore {
			path: self.path.clone(),
			subdir: Some(match &self.subdir {
				Some(existing) => existing.join(&path),
				None => path,
			}),
		})
	}

	fn base(&self) -> Box<dyn BlobStoreProvider> {
		Box::new(SqliteStore::new(self.path.clone()))
	}

	fn id(&self) -> &'static str { "sqlite" }

	fn root_key(&self) -> SmolStr { format!("sqlite:{}", self.path).into() }

	fn subdir(&self) -> RelPath { self.subdir.clone().unwrap_or_default() }

	fn region(&self) -> Option<String> { None }

	fn store_exists(&self) -> SendBoxedFuture<Result<bool>> {
		let path = self.path.clone();
		Box::pin(async move { fs_ext::exists_async(path).await?.xok() })
	}

	/// Create the database file and its `blobs` table.
	///
	/// # Errors
	/// Fails if the file already exists.
	fn store_create(&self) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move {
			if this.store_exists().await? {
				bevybail!("sqlite store `{}` already exists", this.path);
			}
			this.connection().await.map(drop)
		})
	}

	/// Close the pooled connection and delete the file, with the WAL sidecars
	/// a crash may have left beside it.
	///
	/// # Errors
	/// Fails if the file does not exist.
	fn store_remove(&self) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move {
			if !this.store_exists().await? {
				bevybail!("sqlite store `{}` does not exist", this.path);
			}
			this.close().await;
			fs_ext::remove_async(&this.path).await?;
			for sidecar in ["-wal", "-shm"] {
				let sidecar = format!("{}{sidecar}", this.path);
				if fs_ext::exists_async(&sidecar).await? {
					fs_ext::remove_async(&sidecar).await?;
				}
			}
			Ok(())
		})
	}

	fn insert(&self, path: &RelPath, body: Bytes) -> SendBoxedFuture<Result> {
		let key = self.qualify(path);
		self.run(move |conn| {
			conn.execute(
				"INSERT OR REPLACE INTO blobs (path, body) VALUES (?1, ?2)",
				params![key, body.as_ref()],
			)?;
			Ok(())
		})
	}

	fn list(&self) -> SendBoxedFuture<Result<Vec<RelPath>>> {
		let prefix = self.prefix();
		self.run(move |conn| {
			// `substr` rather than `LIKE`: a prefix holding `%` or `_` needs
			// no escaping, and every match is exact
			let mut stmt = conn.prepare(
				"SELECT path FROM blobs WHERE substr(path, 1, ?2) = ?1 \
				 ORDER BY path",
			)?;
			stmt.query_map(
				params![prefix, prefix.chars().count() as i64],
				|row| row.get::<_, String>(0),
			)?
			.map(|path| {
				path.map(|path| RelPath::new(&path[prefix.len()..]))
					.map_err(BevyError::from)
			})
			.collect()
		})
	}

	fn get(&self, path: &RelPath) -> SendBoxedFuture<Result<Bytes>> {
		let key = self.qualify(path);
		self.run(move |conn| {
			conn.query_row(
				"SELECT body FROM blobs WHERE path = ?1",
				params![key],
				|row| row.get::<_, Vec<u8>>(0),
			)
			.optional()?
			.map(Bytes::from)
			.ok_or_else(|| Self::not_found(format!("blob `{key}`")))
		})
	}

	fn exists(&self, path: &RelPath) -> SendBoxedFuture<Result<bool>> {
		let key = self.qualify(path);
		self.run(move |conn| {
			conn.query_row(
				"SELECT 1 FROM blobs WHERE path = ?1",
				params![key],
				|_| Ok(()),
			)
			.optional()?
			.is_some()
			.xok()
		})
	}

	fn remove(&self, path: &RelPath) -> SendBoxedFuture<Result> {
		let key = self.qualify(path);
		self.run(move |conn| {
			match conn
				.execute("DELETE FROM blobs WHERE path = ?1", params![key])?
			{
				0 => Err(Self::not_found(format!("blob `{key}`"))),
				_ => Ok(()),
			}
		})
	}

	fn public_url(
		&self,
		_path: &RelPath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		Box::pin(async move { Ok(None) })
	}
}

/// Native table form: one SQL table per [`TableStoreRow::table_name`],
/// created on first write, the row's canonical json beside its key. Every
/// read answers for a table that does not exist yet as it would for an empty
/// one, so a consumer's first query needs no prior write.
impl TableProvider for SqliteStore {
	fn box_clone_table(&self) -> Box<dyn TableProvider> {
		Box::new(self.clone())
	}

	fn insert_row(
		&self,
		table: &str,
		key: &TableKey,
		row: Value,
	) -> SendBoxedFuture<Result> {
		let table = SmolStr::from(table);
		let key = key.clone();
		self.run(move |conn| {
			let ident = table_ident(&table)?;
			ensure_table(conn, &ident)?;
			conn.execute(
				&format!(
					"INSERT OR REPLACE INTO {ident} (key, json) VALUES (?1, ?2)"
				),
				params![key.as_ref() as &str, canonical_json(row)?],
			)?;
			Ok(())
		})
	}

	/// One transaction for the whole batch, so a segment of thousands of
	/// rows is one commit and the table is never seen half-applied.
	fn insert_rows(
		&self,
		table: &str,
		rows: Vec<(TableKey, Value)>,
	) -> SendBoxedFuture<Result> {
		let table = SmolStr::from(table);
		self.run(move |conn| {
			let ident = table_ident(&table)?;
			ensure_table(conn, &ident)?;
			let tx = conn.unchecked_transaction()?;
			{
				let mut stmt = tx.prepare(&format!(
					"INSERT OR REPLACE INTO {ident} (key, json) VALUES (?1, ?2)"
				))?;
				for (key, row) in rows {
					stmt.execute(params![
						key.as_ref() as &str,
						canonical_json(row)?
					])?;
				}
			}
			tx.commit()?;
			Ok(())
		})
	}

	fn get_row(
		&self,
		table: &str,
		key: &TableKey,
	) -> SendBoxedFuture<Result<Value>> {
		let table = SmolStr::from(table);
		let key = key.clone();
		self.run(move |conn| {
			let ident = table_ident(&table)?;
			let missing = || Self::not_found(format!("row `{table}/{key}`"));
			if !table_exists(conn, &table)? {
				return Err(missing());
			}
			conn.query_row(
				&format!("SELECT json FROM {ident} WHERE key = ?1"),
				params![key.as_ref() as &str],
				|row| row.get::<_, String>(0),
			)
			.optional()?
			.ok_or_else(missing)
			.and_then(|json| serde_json::from_str::<Value>(&json)?.xok())
		})
	}

	/// One prepared select run per key on the blocking thread, so a diff of
	/// a whole segment is one hop rather than one per row.
	fn get_rows(
		&self,
		table: &str,
		keys: Vec<TableKey>,
	) -> SendBoxedFuture<Result<Vec<Option<Value>>>> {
		let table = SmolStr::from(table);
		self.run(move |conn| {
			let ident = table_ident(&table)?;
			if !table_exists(conn, &table)? {
				return Ok(vec![None; keys.len()]);
			}
			let mut stmt = conn
				.prepare(&format!("SELECT json FROM {ident} WHERE key = ?1"))?;
			keys.iter()
				.map(|key| {
					stmt.query_row(params![key.as_ref() as &str], |row| {
						row.get::<_, String>(0)
					})
					.optional()?
					.map(|json| serde_json::from_str::<Value>(&json)?.xok())
					.transpose()
				})
				.collect()
		})
	}

	fn row_exists(
		&self,
		table: &str,
		key: &TableKey,
	) -> SendBoxedFuture<Result<bool>> {
		let table = SmolStr::from(table);
		let key = key.clone();
		self.run(move |conn| {
			let ident = table_ident(&table)?;
			if !table_exists(conn, &table)? {
				return Ok(false);
			}
			conn.query_row(
				&format!("SELECT 1 FROM {ident} WHERE key = ?1"),
				params![key.as_ref() as &str],
				|_| Ok(()),
			)
			.optional()?
			.is_some()
			.xok()
		})
	}

	fn remove_row(
		&self,
		table: &str,
		key: &TableKey,
	) -> SendBoxedFuture<Result> {
		let table = SmolStr::from(table);
		let key = key.clone();
		self.run(move |conn| {
			let ident = table_ident(&table)?;
			let missing = || Self::not_found(format!("row `{table}/{key}`"));
			if !table_exists(conn, &table)? {
				return Err(missing());
			}
			match conn.execute(
				&format!("DELETE FROM {ident} WHERE key = ?1"),
				params![key.as_ref() as &str],
			)? {
				0 => Err(missing()),
				_ => Ok(()),
			}
		})
	}

	fn list_keys(&self, table: &str) -> SendBoxedFuture<Result<Vec<TableKey>>> {
		let table = SmolStr::from(table);
		self.run(move |conn| {
			let ident = table_ident(&table)?;
			if !table_exists(conn, &table)? {
				return Ok(Vec::new());
			}
			conn.prepare(&format!("SELECT key FROM {ident} ORDER BY key"))?
				.query_map([], |row| row.get::<_, String>(0))?
				.map(|key| key.map(TableKey::from).map_err(BevyError::from))
				.collect()
		})
	}

	/// Every row in key order, straight off one select: the listing carries
	/// the bodies, so the N+1 default is never wanted here.
	fn get_all_rows(
		&self,
		table: &str,
	) -> SendBoxedFuture<Result<Vec<(TableKey, Result<Value>)>>> {
		let table = SmolStr::from(table);
		self.run(move |conn| {
			let ident = table_ident(&table)?;
			if !table_exists(conn, &table)? {
				return Ok(Vec::new());
			}
			conn.prepare(&format!(
				"SELECT key, json FROM {ident} ORDER BY key"
			))?
			.query_map([], |row| {
				Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
			})?
			.map(|row| {
				let (key, json) = row?;
				let row = serde_json::from_str::<Value>(&json)
					.map_err(BevyError::from);
				Ok((TableKey::from(key), row))
			})
			.collect()
		})
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A fresh store in a temp dir, the dir kept alive beside it.
	fn temp_store() -> (TempDir, SqliteStore) {
		let dir = TempDir::new_ws().unwrap();
		let store = SqliteStore::new(dir.join("store.db"));
		(dir, store)
	}

	#[beet_core::test]
	async fn store() {
		let (_dir, store) = temp_store();
		store_test::run(store).await;
	}

	#[beet_core::test]
	async fn table() {
		let (_dir, store) = temp_store();
		table_test::run(store).await;
	}

	/// The component hook derives both erased currencies, the table one
	/// wrapping this store's SQL tables.
	#[beet_core::test]
	fn materializes_both_stores() {
		let (_dir, store) = temp_store();
		let mut world = World::new();
		let entity = world.spawn(store).id();
		world.flush();
		world.entity(entity).contains::<BlobStore>().xpect_true();
		world.entity(entity).contains::<TableStore>().xpect_true();
	}

	/// A blob scope is a key prefix, and the base store lists through it.
	#[beet_core::test]
	async fn subdir_scopes_blobs() {
		let (_dir, store) = temp_store();
		let store = BlobStore::new(store);
		let docs = store.with_subdir(RelPath::new("docs"));
		docs.insert(&RelPath::new("a.txt"), "a").await.unwrap();
		store.insert(&RelPath::new("b.txt"), "b").await.unwrap();
		docs.list()
			.await
			.unwrap()
			.xpect_eq(vec![RelPath::new("a.txt")]);
		store
			.list()
			.await
			.unwrap()
			.xpect_eq(vec![RelPath::new("b.txt"), RelPath::new("docs/a.txt")]);
	}

	/// The escape hatch: a view over `json_extract` reads the rows a typed
	/// table wrote, each result row keyed by column name.
	#[beet_core::test]
	async fn query_reads_json_extract() {
		let (_dir, store) = temp_store();
		let table = Table::<TableItem<u32>>::new(store.clone());
		table.push(TableItem::new(7u32)).await.unwrap();
		table.push(TableItem::new(9u32)).await.unwrap();
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
		store
			.query("SELECT count(*) AS n FROM table_item")
			.await
			.unwrap()
			.xpect_eq(vec![Value::Map(Map::new([("n", 2i64)]))]);
	}

	/// Two inserts of the same document with different map insertion order
	/// store identical text, so a diff by text means a diff by content.
	#[beet_core::test]
	async fn canonical_json_is_stable() {
		let (_dir, store) = temp_store();
		let key = TableKey::from("row");
		let first = Value::Map(Map::new([("b", 2), ("a", 1)]));
		let second = Value::Map(Map::new([("a", 1), ("b", 2)]));
		let stored = async |row: Value| {
			store.insert_row("rows", &key, row).await.unwrap();
			store
				.query("SELECT json FROM rows")
				.await
				.unwrap()
				.xmap(|rows| rows[0].clone())
		};
		let text = stored(first).await;
		text.xpect_eq(Value::Map(Map::new([("json", r#"{"a":1,"b":2}"#)])));
		stored(second).await.xpect_eq(text);
	}

	/// A read of a table nothing has written yet is empty, not an error, and
	/// an invalid or reserved table name never reaches the SQL.
	#[beet_core::test]
	async fn reads_before_writes_and_bad_names() {
		let (_dir, store) = temp_store();
		let key = TableKey::from("k");
		store.list_keys("unwritten").await.unwrap().xpect_eq(vec![]);
		store
			.get_all_rows("unwritten")
			.await
			.unwrap()
			.len()
			.xpect_eq(0);
		store
			.row_exists("unwritten", &key)
			.await
			.unwrap()
			.xpect_false();
		store.get_row("unwritten", &key).await.xpect_err();
		store.remove_row("unwritten", &key).await.xpect_err();
		for bad in ["Foo-Bar", "1abc", "", "blobs"] {
			store.insert_row(bad, &key, Value::Null).await.xpect_err();
		}
	}
}
