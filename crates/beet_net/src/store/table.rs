use crate::prelude::*;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use uuid::Uuid;

/// Type-safe storage table for serializable objects.
///
/// # Example
/// ```
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// # async fn run() -> Result<()> {
/// let table = temp_table::<TableItem<String>>();
/// table.store_try_create().await?;
///
/// let item = TableItem::new("Hello, world!".to_string());
/// let id = item.id();
///
/// // Insert and retrieve typed objects
/// table.push(item.clone()).await?;
/// let retrieved = table.get(id).await?;
/// assert_eq!(item.data, retrieved.data);
/// # Ok(())
/// # }
/// ```
#[derive(Component)]
pub struct TableStore<T: TableStoreRow> {
	/// The provider that handles table operations (S3, filesystem, memory, etc).
	provider: Arc<dyn TableProvider<T>>,
}

impl<T: TableStoreRow> Clone for TableStore<T> {
	fn clone(&self) -> Self {
		Self {
			provider: Arc::clone(&self.provider),
		}
	}
}

impl<T: TableStoreRow> TableStore<T> {
	/// Create a new table store with the given provider.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// let table = temp_table::<TableItem<String>>();
	/// ```
	pub fn new(provider: impl TableProvider<T>) -> Self {
		Self {
			provider: Arc::new(provider),
		}
	}

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
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let table = temp_table::<TableItem<String>>();
	/// let item = TableItem::new("test data".to_string());
	/// table.push(item).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn push(&self, body: T) -> Result {
		self.provider.insert_row(body).await
	}

	/// Insert typed object, failing if it already exists.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let table = temp_table::<TableItem<String>>();
	/// let item = TableItem::new("test data".to_string());
	/// table.try_push(item).await?;
	/// # Ok(())
	/// # }
	/// ```
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
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let table = temp_table::<TableItem<String>>();
	/// let item = TableItem::new("test".to_string());
	/// let id = item.id();
	/// let exists = table.exists(id).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn exists(&self, id: Uuid) -> Result<bool> {
		let path = SmolPath::new(id.to_string());
		BlobStoreProvider::exists(self.provider.as_ref(), &path).await
	}

	/// List all object paths in table.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let table = temp_table::<TableItem<String>>();
	/// let paths = table.list().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn list(&self) -> Result<Vec<SmolPath>> {
		BlobStoreProvider::list(self.provider.as_ref()).await
	}

	/// Get typed object data by id.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let table = temp_table::<TableItem<String>>();
	/// let item = TableItem::new("test".to_string());
	/// let id = item.id();
	/// let retrieved = table.get(id).await?;
	/// # Ok(())
	/// # }
	/// ```
	///
	/// # Errors
	/// Returns error if object doesn't exist or fails to deserialize.
	pub async fn get(&self, id: Uuid) -> Result<T> {
		self.provider.get_row(id).await
	}

	/// Get all objects and their typed data.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let table = temp_table::<TableItem<String>>();
	/// let items = table.get_all().await?;
	/// for (path, item) in items {
	///     println!("Item at {}: {}", path, item.data);
	/// }
	/// # Ok(())
	/// # }
	/// ```
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
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let table = temp_table::<TableItem<String>>();
	/// let item = TableItem::new("test".to_string());
	/// let id = item.id();
	/// table.remove(id).await?;
	/// # Ok(())
	/// # }
	/// ```
	///
	/// # Errors
	/// Returns error if object doesn't exist.
	pub async fn remove(&self, id: Uuid) -> Result {
		let path = SmolPath::new(id.to_string());
		BlobStoreProvider::remove(self.provider.as_ref(), &path).await
	}

	/// Get public URL for object (if supported by provider).
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let table = temp_table::<TableItem<String>>();
	/// let path = SmolPath::from("some-uuid");
	/// if let Some(url) = table.public_url(&path).await? {
	///     println!("Public URL: {}", url);
	/// }
	/// # Ok(())
	/// # }
	/// ```
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

/// Types that can be stored in a [`TableStore`].
///
/// This trait is automatically implemented for any type that implements the required bounds:
/// - [`Serialize`] - For encoding objects into bytes
/// - [`DeserializeOwned`] - For decoding objects from bytes
/// - [`Clone`] - For copying objects
/// - `'static` - For type safety across async boundaries
pub trait TableStoreRow: TableContent {
	/// Unique identifier for the object, used as the primary key in the table.
	fn id(&self) -> Uuid;
	/// Decodes uuid timestamp as time since unix epoch.
	/// ## Panics
	/// Panics if uuid is not v1, v6 or v7.
	fn timestamp(&self) -> Duration {
		let timestamp = self.id().get_timestamp().unwrap();
		let (secs, nanos) = timestamp.to_unix();
		Duration::new(secs, nanos)
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
	/// Duration since Unix epoch, from the cross-platform [`time_ext::now`]
	/// (the std `SystemTime` has no wasm clock).
	pub created: Duration,
	/// The user-provided data payload.
	pub data: T,
}

impl<T> TableItem<T> {
	/// Creates a new table item with an auto-generated UUID v7 and current timestamp.
	pub fn new(data: T) -> Self {
		Self {
			id: uuid_ext::now_v7(),
			created: time_ext::now(),
			data,
		}
	}
}
impl<T: TableContent> TableStoreRow for TableItem<T> {
	fn id(&self) -> Uuid { self.id }
}

/// Storage provider for typed table operations.
///
/// This trait extends [`BlobStoreProvider`] with type-safe operations for storing
/// and retrieving serializable objects. Implementors only need to provide
/// [`box_clone_table`](TableProvider::box_clone_table), as the default
/// implementations use the [`BlobStoreProvider`] api to store data as JSON.
pub trait TableProvider<T: TableStoreRow>:
	BlobStoreProvider + 'static + Send + Sync
{
	/// Returns a boxed clone of this provider for type erasure.
	fn box_clone_table(&self) -> Box<dyn TableProvider<T>>;
	/// Inserts a row into the table, serializing it as JSON.
	fn insert_row(&self, body: T) -> SendBoxedFuture<Result> {
		let path = SmolPath::new(body.id().to_string());
		match serde_json::to_vec(&body) {
			Ok(vec) => BlobStoreProvider::insert(self, &path, vec.into()),
			Err(e) => {
				Box::pin(async move { bevybail!("Failed to serialize: {}", e) })
			}
		}
	}
	/// Retrieves a row by its UUID, deserializing from JSON.
	fn get_row(&self, id: Uuid) -> SendBoxedFuture<Result<T>> {
		let path = SmolPath::new(id.to_string());
		let fut = BlobStoreProvider::get(self, &path);
		Box::pin(async move {
			let bytes = fut.await?;
			serde_json::from_slice(&bytes)
				.map_err(|e| bevyhow!("Failed to deserialize: {}", e))
		})
	}
}

/// The [`BlobStore`] wrapper is a [`TableProvider`] for free, serializing rows
/// as JSON via the default trait methods. This is what lets a single
/// [`BlobStore`] back many typed [`TableStore`]s, one per record-type subdir.
#[cfg(all(feature = "json", feature = "std"))]
impl<T: TableStoreRow> TableProvider<T> for BlobStore {
	fn box_clone_table(&self) -> Box<dyn TableProvider<T>> {
		Box::new(self.clone())
	}
}

/// Create temporary in-memory table for testing.
/// The returned table is pre-created and ready for immediate use.
pub fn temp_table<T: TableStoreRow>() -> TableStore<T> {
	TableStore::new(InMemoryStore::new())
}

/// Select filesystem or DynamoDB [`TableProvider`] based on [`ServiceAccess`]
/// and feature flags.
#[allow(unused_variables)]
pub async fn dynamo_fs_selector<T: TableStoreRow>(
	fs_path: &AbsPathBuf,
	table_name: &str,
	region: &str,
	access: ServiceAccess,
) -> TableStore<T> {
	match access {
		ServiceAccess::Local => {
			debug!("Table Selector - FS: {fs_path}");
			TableStore::new(FsStore::new(fs_path.clone()))
		}
		#[cfg(not(all(feature = "aws_sdk", not(target_arch = "wasm32"))))]
		ServiceAccess::Remote => {
			debug!("Table Selector - FS (no aws_sdk feature): {fs_path}");
			TableStore::new(FsStore::new(fs_path.clone()))
		}
		#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
		ServiceAccess::Remote => {
			debug!("Table Selector - Dynamo: {table_name}");
			TableStore::new(DynamoStore::new(table_name, region))
		}
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
	pub async fn run(provider: impl TableProvider<TableItem<MyObject>>) {
		let table = TableStore::new(provider);
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

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A row that fails to deserialize (eg a legacy schema) or has a non-uuid
	/// path is skipped by the lossy read instead of failing the whole scan.
	#[beet_core::test]
	async fn get_all_lossy_skips_unreadable_rows() {
		let provider = InMemoryStore::new();
		let table = TableStore::<TableItem<u32>>::new(provider.clone());
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
