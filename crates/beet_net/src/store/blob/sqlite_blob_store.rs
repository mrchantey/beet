use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::params;

/// Blobs in a SQLite database file, in one `blobs` table, the blob store a
/// `sqlite:<path>` uri names: opted into by declaring it, alone or beside a
/// [`SqliteStore`] on the same path, which shares its connection.
///
/// ```html
/// <SqliteStore path="data.db" {SqliteBlobStore{path:"data.db"}}/>
/// ```
///
/// Its lifecycle is the `blobs` table's, so removing it leaves the rows of a
/// database it shares, while removing the [`SqliteStore`] deletes the file and
/// both with it.
#[derive(Debug, Clone, Component, Get, Reflect)]
#[reflect(Component, Default)]
#[component(on_insert = BlobStore::on_insert::<Self>)]
pub struct SqliteBlobStore {
	/// The database file, see [`SqliteStore::path`].
	path: AbsPath,
	/// Optional key prefix for every blob.
	#[get(skip)]
	#[reflect(default)]
	subdir: Option<RelPath>,
}

/// [`SqliteStore`]'s own default, so the two tags alone share one file.
impl Default for SqliteBlobStore {
	fn default() -> Self { SqliteStore::default().into() }
}

impl From<SqliteStore> for SqliteBlobStore {
	fn from(store: SqliteStore) -> Self { Self::new(store.path().clone()) }
}

impl SqliteBlobStore {
	/// The blobs of the database file `path`, created on first use.
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

	/// The blobs of the database a `sqlite:<path>` uri names, see
	/// [`SqliteStore::from_uri`].
	pub fn from_uri(uri: &StoreUri) -> Result<Self> {
		SqliteStore::from_uri(uri)?.xmap(Self::from).xok()
	}

	/// The database the blobs are in.
	pub fn database(&self) -> SqliteStore {
		SqliteStore::new(self.path.clone())
	}

	/// Run `func` against the database, its `blobs` table there.
	fn run<T: 'static + Send>(
		&self,
		func: impl 'static + Send + FnOnce(&Connection) -> Result<T>,
	) -> SendBoxedFuture<Result<T>> {
		self.database().run(move |conn| {
			Self::ensure(conn)?;
			func(conn)
		})
	}

	fn ensure(conn: &Connection) -> Result {
		conn.execute_batch(
			"CREATE TABLE IF NOT EXISTS blobs (\
				path TEXT PRIMARY KEY, body BLOB NOT NULL)",
		)?;
		Ok(())
	}

	/// Whether the database has its `blobs` table.
	fn has_table(conn: &Connection) -> Result<bool> {
		conn.query_row(
			"SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'blobs'",
			[],
			|_| Ok(()),
		)
		.optional()?
		.is_some()
		.xok()
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

	/// A 404 for a missing blob, matching every other backend so a served
	/// route distinguishes an absent object from a broken store.
	fn not_found(key: &str) -> BevyError {
		HttpError::new(StatusCode::NOT_FOUND, format!("blob `{key}` not found"))
			.into()
	}

	/// The blob at `key` with its digest, `None` where there is none.
	fn versioned_blob(
		conn: &Connection,
		key: &str,
	) -> Result<Option<(Bytes, BlobVersion)>> {
		conn.query_row(
			"SELECT body FROM blobs WHERE path = ?1",
			params![key],
			|row| row.get::<_, Vec<u8>>(0),
		)
		.optional()?
		.map(|body| {
			let version = BlobVersion::of(&body);
			(Bytes::from(body), version)
		})
		.xok()
	}
}

impl StoreBackend for SqliteBlobStore {
	fn id(&self) -> &'static str { "sqlite" }

	fn describe(&self) -> String { self.root_key().to_string() }

	fn region(&self) -> Option<String> { None }

	/// Whether the database is there and has its `blobs` table.
	fn store_exists(&self) -> SendBoxedFuture<Result<bool>> {
		let database = self.database();
		let exists = database.store_exists();
		Box::pin(async move {
			match exists.await? {
				true => database.run(Self::has_table).await,
				false => false.xok(),
			}
		})
	}

	fn store_create(&self) -> SendBoxedFuture<Result> {
		let key = self.root_key();
		self.database().run(move |conn| {
			if Self::has_table(conn)? {
				bevybail!("blob store `{key}` already exists");
			}
			Self::ensure(conn)
		})
	}

	/// Drop the `blobs` table, leaving the file and any rows in it.
	///
	/// # Errors
	/// Fails if the table does not exist.
	fn store_remove(&self) -> SendBoxedFuture<Result> {
		let key = self.root_key();
		let database = self.database();
		let exists = self.store_exists();
		Box::pin(async move {
			if !exists.await? {
				bevybail!("blob store `{key}` does not exist");
			}
			database
				.run(|conn| {
					conn.execute_batch("DROP TABLE blobs")?;
					Ok(())
				})
				.await
		})
	}
}

impl BlobProvider for SqliteBlobStore {
	fn box_clone(&self) -> Box<dyn BlobProvider> { Box::new(self.clone()) }

	fn with_subdir(&self, path: RelPath) -> Box<dyn BlobProvider> {
		Box::new(Self {
			path: self.path.clone(),
			subdir: Some(match &self.subdir {
				Some(existing) => existing.join(&path),
				None => path,
			}),
		})
	}

	fn base(&self) -> Box<dyn BlobProvider> {
		Box::new(Self::new(self.path.clone()))
	}

	fn root_key(&self) -> SmolStr { format!("sqlite:{}", self.path).into() }

	fn subdir(&self) -> RelPath { self.subdir.clone().unwrap_or_default() }

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
			.ok_or_else(|| Self::not_found(&key))
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
				0 => Err(Self::not_found(&key)),
				_ => Ok(()),
			}
		})
	}

	fn conditional_writes(&self) -> bool { true }

	fn get_versioned(
		&self,
		path: &RelPath,
	) -> SendBoxedFuture<Result<Option<(Bytes, BlobVersion)>>> {
		let key = self.qualify(path);
		self.run(move |conn| Self::versioned_blob(conn, &key))
	}

	/// Checked and written under the connection's lock, the version the
	/// content's digest.
	fn insert_if(
		&self,
		path: &RelPath,
		body: Bytes,
		condition: BlobCondition,
	) -> SendBoxedFuture<Result<BlobVersion>> {
		let key = self.qualify(path);
		let path = path.clone();
		self.run(move |conn| {
			let found = Self::versioned_blob(conn, &key)?;
			BlobConflict::check(
				&condition,
				&path,
				found.as_ref().map(|(_, version)| version),
			)?;
			conn.execute(
				"INSERT OR REPLACE INTO blobs (path, body) VALUES (?1, ?2)",
				params![key, body.as_ref()],
			)?;
			BlobVersion::of(&body).xok()
		})
	}

	fn remove_if(
		&self,
		path: &RelPath,
		condition: BlobCondition,
	) -> SendBoxedFuture<Result> {
		let key = self.qualify(path);
		let path = path.clone();
		self.run(move |conn| {
			let found = Self::versioned_blob(conn, &key)?;
			BlobConflict::check(
				&condition,
				&path,
				found.as_ref().map(|(_, version)| version),
			)?;
			conn.execute("DELETE FROM blobs WHERE path = ?1", params![key])?;
			Ok(())
		})
	}

	fn public_url(
		&self,
		_path: &RelPath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		Box::pin(async move { Ok(None) })
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A database of the test's own, in memory under its name.
	fn database(name: &str) -> SqliteStore {
		SqliteStore::memory(format!("sqlite-blobs-{name}"))
	}

	#[beet_core::test]
	async fn store() {
		store_test::run(SqliteBlobStore::from(database("suite"))).await;
	}

	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	async fn store_in_a_file() {
		let dir = TempDir::new_ws().unwrap();
		store_test::run(SqliteBlobStore::new(dir.join("blobs.db"))).await;
	}

	/// The component lands its blob store and no table store.
	#[beet_core::test]
	fn materializes_blobs_alone() {
		let mut world = World::new();
		let entity = world
			.spawn(SqliteBlobStore::from(database("materializes")))
			.id();
		world.flush();
		world.entity(entity).contains::<BlobStore>().xpect_true();
		world.entity(entity).contains::<TableStore>().xpect_false();
	}

	/// Beside a [`SqliteStore`] on the same path the two share the file, each
	/// with its own lifecycle: removing the blobs leaves the rows.
	#[beet_core::test]
	async fn shares_a_database() {
		let rows = database("shares");
		let mut world = World::new();
		let entity = world
			.spawn((rows.clone(), SqliteBlobStore::from(rows)))
			.id();
		world.flush();
		let tables = world.entity(entity).get::<TableStore>().unwrap().clone();
		let blobs = world.entity(entity).get::<BlobStore>().unwrap().clone();
		let table = tables.table::<table_test::NamedRow>();
		table.put(table_test::NamedRow::new("a", 1)).await.unwrap();
		blobs.store_exists().await.unwrap().xpect_false();
		blobs.insert(&RelPath::new("a.txt"), "a").await.unwrap();
		table.put(table_test::NamedRow::new("b", 2)).await.unwrap();
		blobs
			.list()
			.await
			.unwrap()
			.xpect_eq(vec![RelPath::new("a.txt")]);
		blobs.store_remove().await.unwrap();
		blobs.store_exists().await.unwrap().xpect_false();
		table.list().await.unwrap().len().xpect_eq(2);
	}

	/// A blob scope is a key prefix, and the base store lists through it.
	#[beet_core::test]
	async fn subdir_scopes_blobs() {
		let store = BlobStore::new(SqliteBlobStore::from(database("subdir")));
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
}
