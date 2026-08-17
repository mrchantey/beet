use crate::prelude::*;
use beet_core::prelude::bevy_ecs::error::ErrorContext;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;
use std::sync::Arc;
use uuid::Uuid;

/// Type-erased table store: rows stored as [`Value`] documents keyed by [`Uuid`].
///
/// The table twin of [`BlobStore`]: wraps an [`Arc<dyn TableProvider>`] and is
/// materialized onto every store entity by the provider component hooks
/// ([`BlobStore::on_add`] inserts the json-over-blobs form under `json`, a
/// table-native provider like `DynamoStore` overrides it with its own via
/// [`TableStore::on_add`]), so a consumer resolves `TableStore` from an entity
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

	/// A typed view over this store, rows serialized at the edge via [`Value`].
	pub fn table<T: TableStoreRow>(&self) -> Table<T> {
		Table {
			provider: Arc::clone(&self.provider),
			_marker: PhantomData,
		}
	}

	/// Component hook that reads a concrete table provider component from the
	/// entity and inserts a [`TableStore`] wrapping it.
	/// Use with `#[component(on_add = TableStore::on_add::<MyStore>)]`; run
	/// after [`BlobStore::on_add`] it overrides the json-over-blobs table that
	/// hook materializes.
	pub fn on_add<T: Component + Clone + TableProvider>(
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
					name: core::any::type_name_of_val(&TableStore::on_add::<T>)
						.into(),
				});
			}
		}
	}
}

/// Typed view over a [`TableStore`], rows serialized to [`Value`] documents at
/// this edge. The table twin of [`Blob`].
pub struct Table<T: TableStoreRow> {
	provider: Arc<dyn TableProvider>,
	_marker: PhantomData<T>,
}

impl<T: TableStoreRow> Clone for Table<T> {
	fn clone(&self) -> Self {
		Self {
			provider: Arc::clone(&self.provider),
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
	/// let id = item.id();
	///
	/// // insert, retrieve, remove typed objects
	/// table.push(item.clone()).await?;
	/// let retrieved = table.get(id).await?;
	/// assert_eq!(item.data, retrieved.data);
	/// table.remove(id).await?;
	/// # Ok(())
	/// # }
	/// ```
	#[cfg(feature = "json")]
	pub fn temp() -> Self { TableStore::temp().table() }

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

	/// Insert typed object into table.
	pub async fn push(&self, body: T) -> Result {
		let id = body.id();
		self.provider.insert_row(id, Value::from_serde(body)?).await
	}

	/// Insert typed object, failing if it already exists.
	///
	/// # Errors
	/// Returns error if object already exists at path.
	pub async fn try_push(&self, body: T) -> Result {
		let id = body.id();
		if self.exists(id).await? {
			bevybail!("Row already exists: {}", id)
		} else {
			self.push(body).await
		}
	}

	/// Check if object exists at path.
	pub async fn exists(&self, id: Uuid) -> Result<bool> {
		let path = SmolPath::new(id.to_string());
		BlobStoreProvider::exists(self.provider.as_ref(), &path).await
	}

	/// List all object paths in table.
	pub async fn list(&self) -> Result<Vec<SmolPath>> {
		BlobStoreProvider::list(self.provider.as_ref()).await
	}

	/// Get typed object data by id.
	///
	/// # Errors
	/// Returns error if object doesn't exist or fails to deserialize.
	pub async fn get(&self, id: Uuid) -> Result<T> {
		self.provider.get_row(id).await?.into_serde()
	}

	/// Get all objects and their typed data.
	///
	/// # Caution
	/// Expensive operation - prefer [`Self::list`] + [`Self::get`] for large tables.
	pub async fn get_all(&self) -> Result<Vec<(SmolPath, T)>> {
		self.list()
			.await?
			.into_iter()
			.map(async |path| {
				let id = path.to_string().parse::<Uuid>().map_err(|e| {
					bevyhow!("Invalid UUID in path {}: {}", path, e)
				})?;
				let data = self.get(id).await?;
				Ok::<_, BevyError>((path, data))
			})
			.xmap(async_ext::try_join_all)
			.await
	}

	/// Like [`Self::get_all`], but a row that fails to parse or deserialize is
	/// skipped with a warning instead of failing the whole read.
	///
	/// Prefer for telemetry-style tables (eg analytics) where a legacy-schema or
	/// corrupt row must not brick every aggregate query over the table.
	pub async fn get_all_lossy(&self) -> Result<Vec<(SmolPath, T)>> {
		self.list()
			.await?
			.into_iter()
			.map(async |path| {
				let row = match path.to_string().parse::<Uuid>() {
					Ok(id) => match self.get(id).await {
						Ok(row) => Some((path, row)),
						Err(err) => {
							warn!("skipping unreadable row {path}: {err}");
							None
						}
					},
					Err(err) => {
						warn!("skipping non-uuid row {path}: {err}");
						None
					}
				};
				Ok::<_, BevyError>(row)
			})
			.xmap(async_ext::try_join_all)
			.await
			.map(|rows| rows.into_iter().flatten().collect())
	}

	/// Remove object from table by id.
	///
	/// # Errors
	/// Returns error if object doesn't exist.
	pub async fn remove(&self, id: Uuid) -> Result {
		let path = SmolPath::new(id.to_string());
		BlobStoreProvider::remove(self.provider.as_ref(), &path).await
	}

	/// Get public URL for object (if supported by provider).
	///
	/// Returns `None` if provider doesn't support public URLs.
	pub async fn public_url(&self, path: &SmolPath) -> Result<Option<String>> {
		BlobStoreProvider::public_url(self.provider.as_ref(), path).await
	}

	/// Get provider region.
	pub fn region(&self) -> Option<String> {
		BlobStoreProvider::region(self.provider.as_ref())
	}
}

/// Types that can be stored in a [`Table`].
///
/// This trait is automatically implemented for any type that implements the required bounds:
/// - [`Serialize`] - For encoding objects into bytes
/// - [`DeserializeOwned`] - For decoding objects from bytes
/// - [`Clone`] - For copying objects
/// - `'static` - For type safety across async boundaries
///
/// The serialized row must carry its [`id`](Self::id) as an `id` field, the
/// primary key a table-native backend (eg DynamoDB) retrieves by.
pub trait TableStoreRow: TableContent {
	/// Unique identifier for the object, used as the primary key in the table.
	fn id(&self) -> Uuid;
	/// Decodes the uuid's embedded wall-clock time.
	/// ## Panics
	/// Panics if uuid is not v1, v6 or v7.
	fn timestamp(&self) -> Timestamp {
		let timestamp = self.id().get_timestamp().unwrap();
		let (secs, nanos) = timestamp.to_unix();
		Timestamp::from_unix_epoch_elapsed(Duration::new(secs, nanos))
	}
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
	fn id(&self) -> Uuid { self.id }
}

/// Storage provider for table operations over untyped [`Value`] rows.
///
/// Extends [`BlobStoreProvider`] with document operations, and is deliberately
/// encoding-agnostic: only the [`BlobStore`] impl (under `json`) knows about
/// bytes, encoding rows as JSON so any blob store backs a table; a table-native
/// backend like `DynamoStore` stores structured documents directly.
pub trait TableProvider: BlobStoreProvider + 'static + Send + Sync {
	/// Returns a boxed clone of this provider for type erasure.
	fn box_clone_table(&self) -> Box<dyn TableProvider>;
	/// Insert the row document at `id`.
	fn insert_row(&self, id: Uuid, row: Value) -> SendBoxedFuture<Result>;
	/// Get the row document at `id`.
	fn get_row(&self, id: Uuid) -> SendBoxedFuture<Result<Value>>;
}

/// The [`BlobStore`] wrapper is a [`TableProvider`] for free, encoding rows as
/// JSON bytes at their id: this is what lets any blob store back a table, and a
/// single [`BlobStore`] back many typed [`Table`]s, one per record-type subdir.
/// The one impl that knows about bytes; a native beet [`Value`] codec would
/// swap in here.
#[cfg(feature = "json")]
impl TableProvider for BlobStore {
	fn box_clone_table(&self) -> Box<dyn TableProvider> {
		Box::new(self.clone())
	}

	fn insert_row(&self, id: Uuid, row: Value) -> SendBoxedFuture<Result> {
		let path = SmolPath::new(id.to_string());
		match serde_json::to_vec(&row) {
			Ok(bytes) => BlobStoreProvider::insert(self, &path, bytes.into()),
			Err(e) => {
				Box::pin(async move { bevybail!("Failed to serialize: {}", e) })
			}
		}
	}

	fn get_row(&self, id: Uuid) -> SendBoxedFuture<Result<Value>> {
		let path = SmolPath::new(id.to_string());
		let fut = BlobStoreProvider::get(self, &path);
		Box::pin(async move {
			let bytes = fut.await?;
			serde_json::from_slice(&bytes)
				.map_err(|e| bevyhow!("Failed to deserialize: {}", e))
		})
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

	/// Runs the standard table provider test suite.
	pub async fn run(provider: impl TableProvider) {
		let table = Table::<TableItem<MyObject>>::new(provider);
		let body = TableItem::new(MyObject {
			some_key: "some_value".into(),
			some_vec: vec![MyObject {
				some_key: "nested".into(),
				some_vec: vec![],
			}],
		});
		let id = body.id();
		let path = SmolPath::new(id.to_string());
		table.store_remove().await.ok();
		table.store_exists().await.unwrap().xpect_false();
		table.store_try_create().await.unwrap();
		table.exists(id).await.unwrap().xpect_false();
		table.remove(id).await.xpect_err();
		table.push(body.clone()).await.unwrap();
		table.store_exists().await.unwrap().xpect_true();
		table.exists(id).await.unwrap().xpect_true();
		table.list().await.unwrap().xpect_eq(vec![path.clone()]);
		table.get(id).await.unwrap().xpect_eq(body.clone());
		table.get(id).await.unwrap().xpect_eq(body);

		table.remove(id).await.unwrap();
		table.get(id).await.xpect_err();

		table.store_remove().await.unwrap();
		table.store_exists().await.unwrap().xpect_false();
	}
}

#[cfg(all(test, feature = "json"))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Any provider component materializes the erased [`TableStore`] alongside
	/// [`BlobStore`] via its `on_add` hooks.
	#[beet_core::test]
	fn provider_component_materializes_table_store() {
		let mut world = World::new();
		let entity = world.spawn(InMemoryStore::new()).id();
		world.flush();
		world.entity(entity).contains::<BlobStore>().xpect_true();
		world.entity(entity).contains::<TableStore>().xpect_true();
	}

	/// A row that fails to deserialize (eg a legacy schema) or has a non-uuid
	/// path is skipped by the lossy read instead of failing the whole scan.
	#[beet_core::test]
	async fn get_all_lossy_skips_unreadable_rows() {
		let provider = InMemoryStore::new();
		let table =
			Table::<TableItem<u32>>::new(BlobStore::new(provider.clone()));
		table.store_try_create().await.unwrap();
		let valid = TableItem::new(7u32);
		let valid_id = valid.id();
		table.push(valid).await.unwrap();
		// a legacy-schema row: a valid uuid path with an undecodable body.
		BlobStoreProvider::insert(
			&provider,
			&SmolPath::new(uuid_ext::now_v7().to_string()),
			r#"{"schema":"legacy"}"#.into(),
		)
		.await
		.unwrap();
		// a non-uuid path.
		BlobStoreProvider::insert(&provider, &SmolPath::new("junk"), "{}".into())
			.await
			.unwrap();

		// the strict read fails, the lossy read yields only the valid row.
		table.get_all().await.xpect_err();
		let rows = table.get_all_lossy().await.unwrap();
		rows.len().xpect_eq(1);
		rows[0].1.id.xpect_eq(valid_id);
	}
}
