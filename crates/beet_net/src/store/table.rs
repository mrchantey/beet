use crate::prelude::*;
use beet_core::prelude::bevy_ecs::error::ErrorContext;
use beet_core::prelude::*;
use heck::ToSnakeCase;
use serde::Deserialize;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;
use std::sync::Arc;
use uuid::Uuid;

/// Type-erased table store: rows stored as [`Value`] documents, keyed by a
/// [`TableKey`] within their [`table_name`](TableStoreRow::table_name).
///
/// The table twin of [`BlobStore`]: wraps an [`Arc<dyn TableProvider>`] and is
/// materialized onto every store entity by the provider component hooks
/// ([`BlobStore::on_insert`] inserts the json-over-blobs form under `json`, a
/// table-native provider like [`DynamoStore`] overrides it with its own via
/// [`TableStore::on_insert`]), so a consumer resolves `TableStore` from an entity
/// and never names a backend. Typed access goes through [`Self::table`],
/// mirroring [`BlobStore::blob`].
#[derive(Clone, Component)]
pub struct TableStore {
	/// The provider that handles table operations (DynamoDB, filesystem, memory, etc).
	provider: Arc<dyn TableProvider>,
}

impl core::fmt::Debug for TableStore {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("TableStore").finish_non_exhaustive()
	}
}

impl TableStore {
	/// Creates a new store wrapping the given provider.
	pub fn new(provider: impl TableProvider) -> Self {
		Self {
			provider: Arc::new(provider),
		}
	}

	/// Create temporary in-memory table store for testing.
	/// The returned store is pre-created and ready for immediate use.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let table = TableStore::temp().table::<TableItem<String>>();
	/// table.push(TableItem::new("Hello, world!".to_string())).await?;
	/// # Ok(())
	/// # }
	/// ```
	#[cfg(feature = "json")]
	pub fn temp() -> Self { Self::new(BlobStore::temp()) }

	/// A typed view over one table of this store, rows serialized at the edge
	/// via [`Value`].
	pub fn table<T: TableStoreRow>(&self) -> Table<T> {
		Table {
			provider: Arc::clone(&self.provider),
			name: T::table_name(),
			_marker: PhantomData,
		}
	}

	/// Component hook that reads a concrete table provider component from the
	/// entity and inserts a [`TableStore`] wrapping it.
	/// Use with `#[component(on_insert = TableStore::on_insert::<MyStore>)]`; run
	/// after [`BlobStore::on_insert`] it overrides the json-over-blobs table that
	/// hook materializes.
	pub fn on_insert<T: Component + Clone + TableProvider>(
		mut world: DeferredWorld,
		cx: HookContext,
	) {
		match world.entity(cx.entity).get_or_else::<T>().cloned() {
			Ok(provider) => {
				world
					.commands()
					.entity(cx.entity)
					.insert(TableStore::new(provider));
			}
			Err(err) => {
				world.fallback_error_handler()(err, ErrorContext::Command {
					name: core::any::type_name_of_val(
						&TableStore::on_insert::<T>,
					)
					.into(),
				});
			}
		}
	}
}

/// Typed view over one table of a [`TableStore`], rows serialized to [`Value`]
/// documents at this edge. The table twin of [`Blob`].
pub struct Table<T: TableStoreRow> {
	provider: Arc<dyn TableProvider>,
	/// [`TableStoreRow::table_name`], resolved once.
	name: SmolStr,
	_marker: PhantomData<T>,
}

impl<T: TableStoreRow> Clone for Table<T> {
	fn clone(&self) -> Self {
		Self {
			provider: Arc::clone(&self.provider),
			name: self.name.clone(),
			_marker: PhantomData,
		}
	}
}

impl<T: TableStoreRow> Table<T> {
	/// Create a new table with the given provider.
	pub fn new(provider: impl TableProvider) -> Self {
		TableStore::new(provider).table()
	}

	/// Create temporary in-memory table for testing.
	/// The returned table is pre-created and ready for immediate use.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let table = Table::<TableItem<String>>::temp();
	/// table.store_try_create().await?;
	///
	/// let item = TableItem::new("Hello, world!".to_string());
	/// let key = item.key();
	///
	/// // insert, retrieve, remove typed objects
	/// table.push(item.clone()).await?;
	/// let retrieved = table.get(key.clone()).await?;
	/// assert_eq!(item.data, retrieved.data);
	/// table.remove(key).await?;
	/// # Ok(())
	/// # }
	/// ```
	#[cfg(feature = "json")]
	pub fn temp() -> Self { TableStore::temp().table() }

	/// The table's name, [`TableStoreRow::table_name`].
	pub fn name(&self) -> &str { &self.name }

	/// Create store (may take 10+ seconds for cloud providers).
	///
	/// # Errors
	/// Fails if store already exists.
	pub async fn store_create(&self) -> Result {
		BlobStoreProvider::store_create(self.provider.as_ref()).await
	}

	/// Ensure store exists, creating if needed.
	pub async fn store_try_create(&self) -> Result {
		BlobStoreProvider::store_try_create(self.provider.as_ref()).await
	}

	/// Check if store exists.
	pub async fn store_exists(&self) -> Result<bool> {
		BlobStoreProvider::store_exists(self.provider.as_ref()).await
	}

	/// Remove store.
	///
	/// # Errors
	/// Fails if store doesn't exist.
	pub async fn store_remove(&self) -> Result {
		BlobStoreProvider::store_remove(self.provider.as_ref()).await
	}

	/// Insert typed row, replacing any row at its key.
	///
	/// # Errors
	/// Fails on an empty key: a row must know what it is called.
	pub async fn push(&self, body: T) -> Result {
		let key = body.key();
		if key.is_empty() {
			bevybail!("empty key for a `{}` row", self.name)
		}
		self.provider
			.insert_row(&self.name, &key, Value::from_serde(body)?)
			.await
	}

	/// Insert typed row, failing if one already exists at its key.
	///
	/// # Errors
	/// Returns error if row already exists.
	pub async fn try_push(&self, body: T) -> Result {
		let key = body.key();
		if self.exists(key.clone()).await? {
			bevybail!("row already exists: {}/{key}", self.name)
		} else {
			self.push(body).await
		}
	}

	/// Check if a row exists at `key`.
	pub async fn exists(&self, key: impl Into<TableKey>) -> Result<bool> {
		self.provider.row_exists(&self.name, &key.into()).await
	}

	/// Every key in the table.
	pub async fn list(&self) -> Result<Vec<TableKey>> {
		self.provider.list_keys(&self.name).await
	}

	/// Get typed row by key.
	///
	/// # Errors
	/// Returns error if row doesn't exist or fails to deserialize.
	pub async fn get(&self, key: impl Into<TableKey>) -> Result<T> {
		self.provider
			.get_row(&self.name, &key.into())
			.await?
			.into_serde()
	}

	/// Get all rows and their typed data.
	///
	/// # Caution
	/// Expensive operation - prefer [`Self::list`] + [`Self::get`] for large tables.
	pub async fn get_all(&self) -> Result<Vec<(TableKey, T)>> {
		self.provider
			.get_all_rows(&self.name)
			.await?
			.into_iter()
			.map(|(key, row)| Ok((key, row?.into_serde()?)))
			.collect()
	}

	/// Like [`Self::get_all`], but a row that fails to read, parse or
	/// deserialize is skipped with a warning instead of failing the whole read.
	///
	/// Prefer for telemetry-style tables (eg analytics) where a legacy-schema or
	/// corrupt row must not brick every aggregate query over the table.
	///
	/// # Caution
	/// A skipped row is silently missing from the result, so a caller reporting
	/// aggregates over this should not present the count as the table's true
	/// total.
	pub async fn get_all_lossy(&self) -> Result<Vec<(TableKey, T)>> {
		self.provider
			.get_all_rows(&self.name)
			.await?
			.into_iter()
			.filter_map(|(key, row)| {
				match row.and_then(|row| row.into_serde::<T>()) {
					Ok(row) => Some((key, row)),
					Err(err) => {
						warn!(
							"skipping unreadable row {}/{key}: {err}",
							self.name
						);
						None
					}
				}
			})
			.collect::<Vec<_>>()
			.xok()
	}

	/// Remove the row at `key`.
	///
	/// # Errors
	/// Returns error if row doesn't exist.
	pub async fn remove(&self, key: impl Into<TableKey>) -> Result {
		self.provider.remove_row(&self.name, &key.into()).await
	}

	/// Get public URL for the row at `key` (if supported by provider).
	///
	/// Returns `None` if provider doesn't support public URLs.
	pub async fn public_url(
		&self,
		key: impl Into<TableKey>,
	) -> Result<Option<String>> {
		let path = key.into().path(&self.name);
		BlobStoreProvider::public_url(self.provider.as_ref(), &path).await
	}

	/// Get provider region.
	pub fn region(&self) -> Option<String> {
		BlobStoreProvider::region(self.provider.as_ref())
	}

	/// Where this table actually lives, ie `s3:beet-site--prod--analytics
	/// (us-west-2)`. The one thing an operator needs from a store that will not
	/// answer, so it belongs in any error naming this table.
	pub fn describe(&self) -> String {
		BlobStoreProvider::describe(self.provider.as_ref())
	}
}

/// A row's primary key within its table: any non-empty string, the source's
/// natural id where it has one. Slashes are allowed, and the blob adapters
/// store them as nested paths.
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	Hash,
	PartialOrd,
	Ord,
	Serialize,
	Deserialize,
	Reflect,
)]
#[serde(transparent)]
pub struct TableKey(SmolStr);

impl TableKey {
	/// Where the blob adapters keep this key's row: `{table}/{key}`.
	pub fn path(&self, table: &str) -> RelPath {
		RelPath::new(table).join(&self.0)
	}
}

impl core::ops::Deref for TableKey {
	type Target = str;
	fn deref(&self) -> &str { &self.0 }
}

impl core::fmt::Display for TableKey {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		self.0.fmt(f)
	}
}

impl From<&str> for TableKey {
	fn from(key: &str) -> Self { Self(key.into()) }
}
impl From<String> for TableKey {
	fn from(key: String) -> Self { Self(key.into()) }
}
impl From<SmolStr> for TableKey {
	fn from(key: SmolStr) -> Self { Self(key) }
}
/// The hyphenated form, ie `0192f8a0-beef-7a11-9a11-a11a7ce50011`.
impl From<Uuid> for TableKey {
	fn from(id: Uuid) -> Self { Self(id.to_string().into()) }
}

/// Types that can be stored in a [`Table`].
///
/// This trait is implemented for any type that implements the required bounds:
/// - [`Serialize`] - For encoding objects into bytes
/// - [`DeserializeOwned`] - For decoding objects from bytes
/// - [`Clone`] - For copying objects
/// - `'static` - For type safety across async boundaries
///
/// The serialized row is stored under its [`key`](Self::key) in its
/// [`table_name`](Self::table_name).
pub trait TableStoreRow: TableContent {
	/// The table this row type lives in, ie `youtube_likes`: by default the
	/// snake case of the short type name, generics dropped, so `TableItem<u32>`
	/// lives in `table_item`.
	fn table_name() -> SmolStr {
		let name = type_ext::short_name::<Self>();
		name.split_once('<')
			.map_or(name.as_str(), |(base, _generics)| base)
			.to_snake_case()
			.into()
	}
	/// The row's primary key.
	fn key(&self) -> TableKey;
	/// When the row was created, where the row knows.
	fn timestamp(&self) -> Option<Timestamp> { None }
}
/// Helper blanket trait constraining types which may be included in a table.
pub trait TableContent:
	'static + Send + Sync + Clone + Serialize + DeserializeOwned
{
}
impl<T> TableContent for T where
	T: 'static + Send + Sync + Clone + Serialize + DeserializeOwned
{
}

/// Helper type implementing [`TableStoreRow`]. Note some services
/// like DynamoDB do not allow indexing nested values, so if thats required
/// a standalone [`TableStoreRow`] implementation should be used.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TableItem<T> {
	/// A uuid v7 used as the primary key.
	pub id: Uuid,
	/// Wall-clock creation time. Deliberately a [`Timestamp`] rather than an
	/// [`Instant`]: that clock is monotonic (elapsed from an arbitrary
	/// process-local zero), so it is meaningless once this row is serialized and
	/// read back in another process.
	pub created: Timestamp,
	/// The user-provided data payload.
	pub data: T,
}

impl<T> TableItem<T> {
	/// Creates a new table item with an auto-generated UUID v7 and current timestamp.
	pub fn new(data: T) -> Self {
		Self {
			id: uuid_ext::now_v7(),
			created: Timestamp::now(),
			data,
		}
	}
}
impl<T: TableContent> TableStoreRow for TableItem<T> {
	fn key(&self) -> TableKey { self.id.into() }
	fn timestamp(&self) -> Option<Timestamp> { Some(self.created) }
}

/// Storage provider for table operations over untyped [`Value`] rows.
///
/// Extends [`BlobStoreProvider`] with document operations, and is deliberately
/// encoding-agnostic: only the [`BlobStore`] impl (under `json`) knows about
/// bytes, encoding rows as JSON so any blob store backs a table; a table-native
/// backend like [`DynamoStore`] stores structured documents directly.
///
/// Every operation names its `table` ([`TableStoreRow::table_name`]) and
/// `key`; the defaults answer through the blob side at [`TableKey::path`],
/// which a table-native backend overrides wholesale.
pub trait TableProvider: BlobStoreProvider + 'static + Send + Sync {
	/// Returns a boxed clone of this provider for type erasure.
	fn box_clone_table(&self) -> Box<dyn TableProvider>;
	/// Insert the row document at `key` in `table`, replacing any there.
	fn insert_row(
		&self,
		table: &str,
		key: &TableKey,
		row: Value,
	) -> SendBoxedFuture<Result>;
	/// Get the row document at `key` in `table`.
	fn get_row(
		&self,
		table: &str,
		key: &TableKey,
	) -> SendBoxedFuture<Result<Value>>;

	/// Whether a row exists at `key` in `table`.
	fn row_exists(
		&self,
		table: &str,
		key: &TableKey,
	) -> SendBoxedFuture<Result<bool>> {
		BlobStoreProvider::exists(self, &key.path(table))
	}

	/// Remove the row at `key` in `table`, erroring if there is none.
	fn remove_row(
		&self,
		table: &str,
		key: &TableKey,
	) -> SendBoxedFuture<Result> {
		BlobStoreProvider::remove(self, &key.path(table))
	}

	/// Every key in `table`.
	fn list_keys(&self, table: &str) -> SendBoxedFuture<Result<Vec<TableKey>>> {
		let scoped = self.with_subdir(RelPath::new(table));
		Box::pin(async move {
			scoped
				.list()
				.await?
				.into_iter()
				.map(|path| TableKey::from(path.as_str()))
				.collect::<Vec<_>>()
				.xok()
		})
	}

	/// Every row in `table`, each paired with the document it read or the
	/// error that row failed with.
	///
	/// Row-level errors are carried rather than raised so the caller picks the
	/// policy: [`Table::get_all`] fails on the first, [`Table::get_all_lossy`]
	/// skips it.
	///
	/// The default lists keys and fetches each row, bounded by
	/// [`BlobStore::GET_ALL_CONCURRENCY`]. A provider whose listing already
	/// carries row bodies should override this to avoid an N+1 over the network.
	fn get_all_rows(
		&self,
		table: &str,
	) -> SendBoxedFuture<Result<Vec<(TableKey, Result<Value>)>>> {
		let this = self.box_clone_table();
		let table = SmolStr::from(table);
		Box::pin(async move {
			this.list_keys(&table)
				.await?
				.into_iter()
				.map(async |key| {
					let row = this.get_row(&table, &key).await;
					(key, row)
				})
				.xmap(|rows| {
					async_ext::join_all_bounded(
						BlobStore::GET_ALL_CONCURRENCY,
						rows,
					)
				})
				.await
				.xok()
		})
	}
}

/// The [`BlobStore`] wrapper is a [`TableProvider`] for free, encoding rows as
/// JSON bytes at [`TableKey::path`]: this is what lets any blob store back a
/// table, and a single [`BlobStore`] back many typed [`Table`]s, one per
/// table subdir. The one impl that knows about bytes; a native beet [`Value`]
/// codec would swap in here.
#[cfg(feature = "json")]
impl TableProvider for BlobStore {
	fn box_clone_table(&self) -> Box<dyn TableProvider> {
		Box::new(self.clone())
	}

	fn insert_row(
		&self,
		table: &str,
		key: &TableKey,
		row: Value,
	) -> SendBoxedFuture<Result> {
		let path = key.path(table);
		match serde_json::to_vec(&row) {
			Ok(bytes) => BlobStoreProvider::insert(self, &path, bytes.into()),
			Err(err) => Box::pin(async move { Err(err.into()) }),
		}
	}

	fn get_row(
		&self,
		table: &str,
		key: &TableKey,
	) -> SendBoxedFuture<Result<Value>> {
		let fut = BlobStoreProvider::get(self, &key.path(table));
		Box::pin(
			async move { serde_json::from_slice::<Value>(&fut.await?)?.xok() },
		)
	}
}

/// Test utilities for table providers.
#[cfg(test)]
pub mod table_test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use serde::Deserialize;
	use serde::Serialize;

	/// Test object for table provider tests.
	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
	pub struct MyObject {
		some_key: String,
		some_vec: Vec<MyObject>,
	}

	/// A row keyed by a string it carries, in a table it names itself.
	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
	pub struct NamedRow {
		key: String,
		value: u32,
	}

	impl TableStoreRow for NamedRow {
		fn table_name() -> SmolStr { "custom_rows".into() }
		fn key(&self) -> TableKey { self.key.as_str().into() }
	}

	/// Runs the standard table provider test suite: the uuid-keyed
	/// [`TableItem`] in its default table, then a string key containing a
	/// slash in a custom-named table, the two disjoint.
	pub async fn run(provider: impl TableProvider) {
		let store = TableStore::new(provider);
		let items = store.table::<TableItem<MyObject>>();
		items.name().xpect_eq("table_item");
		let body = TableItem::new(MyObject {
			some_key: "some_value".into(),
			some_vec: vec![MyObject {
				some_key: "nested".into(),
				some_vec: vec![],
			}],
		});
		let key = body.key();
		items.store_remove().await.ok();
		items.store_exists().await.unwrap().xpect_false();
		items.store_try_create().await.unwrap();
		items.exists(key.clone()).await.unwrap().xpect_false();
		items.remove(key.clone()).await.xpect_err();
		items.push(body.clone()).await.unwrap();
		items.store_exists().await.unwrap().xpect_true();
		items.exists(key.clone()).await.unwrap().xpect_true();
		items.list().await.unwrap().xpect_eq(vec![key.clone()]);
		items.get(key.clone()).await.unwrap().xpect_eq(body.clone());
		items
			.get_all()
			.await
			.unwrap()
			.xpect_eq(vec![(key.clone(), body)]);
		items.remove(key.clone()).await.unwrap();
		items.get(key).await.xpect_err();

		// a slash in the key nests, in the row type's own table
		let rows = store.table::<NamedRow>();
		rows.name().xpect_eq("custom_rows");
		let row = NamedRow {
			key: "2026/09/18".into(),
			value: 7,
		};
		rows.push(row.clone()).await.unwrap();
		rows.list().await.unwrap().xpect_eq(vec![row.key()]);
		rows.get("2026/09/18").await.unwrap().xpect_eq(row.clone());
		rows.get_all()
			.await
			.unwrap()
			.xpect_eq(vec![(row.key(), row.clone())]);
		// tables are disjoint
		items.list().await.unwrap().xpect_eq(Vec::new());
		// the same key with new content is an update
		let updated = NamedRow {
			value: 8,
			..row.clone()
		};
		rows.push(updated.clone()).await.unwrap();
		rows.get(row.key()).await.unwrap().xpect_eq(updated);
		rows.try_push(row.clone()).await.xpect_err();
		rows.remove(row.key()).await.unwrap();
		rows.list().await.unwrap().xpect_eq(Vec::new());

		items.store_remove().await.unwrap();
		items.store_exists().await.unwrap().xpect_false();
	}
}

#[cfg(all(test, feature = "json"))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Any provider component materializes the erased [`TableStore`] alongside
	/// [`BlobStore`] via its `on_insert` hooks.
	#[beet_core::test]
	fn provider_component_materializes_table_store() {
		let mut world = World::new();
		let entity = world.spawn(InMemoryStore::new()).id();
		world.flush();
		world.entity(entity).contains::<BlobStore>().xpect_true();
		world.entity(entity).contains::<TableStore>().xpect_true();
	}

	/// The json-over-blobs adapter passes the shared suite.
	#[beet_core::test]
	async fn blob_adapter() { table_test::run(BlobStore::temp()).await }

	/// A row's default table is the snake case of its type, generics dropped,
	/// and its rows land under it.
	#[beet_core::test]
	async fn default_table_name() {
		TableItem::<Vec<u32>>::table_name().xpect_eq("table_item");
		let store = BlobStore::temp();
		let table = Table::<TableItem<u32>>::new(store.clone());
		let item = TableItem::new(7u32);
		table.push(item.clone()).await.unwrap();
		BlobStoreProvider::list(&store)
			.await
			.unwrap()
			.xpect_eq(vec![RelPath::new(format!("table_item/{}", item.id))]);
	}

	/// A row that fails to deserialize (eg a legacy schema) is skipped by the
	/// lossy read instead of failing the whole scan, and another table's rows
	/// are never read at all.
	#[beet_core::test]
	async fn get_all_lossy_skips_unreadable_rows() {
		let provider = InMemoryStore::new();
		let table =
			Table::<TableItem<u32>>::new(BlobStore::new(provider.clone()));
		table.store_try_create().await.unwrap();
		let valid = TableItem::new(7u32);
		let valid_key = valid.key();
		table.push(valid).await.unwrap();
		// a legacy-schema row: a valid key with an undecodable body.
		BlobStoreProvider::insert(
			&provider,
			&TableKey::from(uuid_ext::now_v7()).path(table.name()),
			r#"{"schema":"legacy"}"#.into(),
		)
		.await
		.unwrap();
		// another table's row.
		BlobStoreProvider::insert(
			&provider,
			&RelPath::new("other/junk"),
			"{}".into(),
		)
		.await
		.unwrap();

		// the strict read fails, the lossy read yields only the valid row.
		table.get_all().await.xpect_err();
		let rows = table.get_all_lossy().await.unwrap();
		rows.len().xpect_eq(1);
		rows[0].0.xpect_eq(valid_key);
	}

	/// A whole-table read spanning more rows than [`BlobStore::GET_ALL_CONCURRENCY`]
	/// returns every one of them: the fan-out is bounded, never truncated.
	#[beet_core::test]
	async fn get_all_reads_past_the_concurrency_limit() {
		let table = Table::<TableItem<u32>>::temp();
		let total = BlobStore::GET_ALL_CONCURRENCY * 3 + 1;
		for idx in 0..total {
			table.push(TableItem::new(idx as u32)).await.unwrap();
		}
		table.get_all().await.unwrap().len().xpect_eq(total);
		table.get_all_lossy().await.unwrap().len().xpect_eq(total);
	}
}
