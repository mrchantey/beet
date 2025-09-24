use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::time::SystemTime;
use uuid::Uuid;


/// Type-safe storage table for serializable objects
///
/// # Example
/// ```
/// # use bevy::prelude::*;
/// # use beet_net::prelude::*;
/// # async fn run() -> Result<()> {
/// let table = temp_table::<TableItem<String>>();
/// table.bucket_try_create().await?;
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
pub struct TableStore<T: TableRow> {
	/// The resource name of the table bucket
	name: String,
	/// The provider that handles table operations (S3, filesystem, memory, etc)
	provider: Box<dyn TableProvider<T>>,
}

impl<T: TableRow> Clone for TableStore<T> {
	fn clone(&self) -> Self {
		Self {
			name: self.name.clone(),
			provider: self.provider.box_clone_table(),
		}
	}
}

impl<T: TableRow> TableStore<T> {
	/// Create a new table store with the given provider and name
	///
	/// # Example
	/// ```
	/// # use bevy::prelude::*;
	/// # use beet_net::prelude::*;
	/// let table = temp_table::<TableItem<String>>();
	/// ```
	pub fn new(
		provider: impl TableProvider<T>,
		name: impl Into<String>,
	) -> Self {
		Self {
			name: name.into(),
			provider: Box::new(provider),
		}
	}

	/// Get table name
	///
	/// # Example
	/// ```
	/// # use bevy::prelude::*;
	/// # use beet_net::prelude::*;
	/// let table = temp_table::<TableItem<String>>();
	/// assert_eq!(table.name(), "temp");
	/// ```
	pub fn name(&self) -> &str { &self.name }

	/// Create bucket (may take 10+ seconds for cloud providers)
	///
	/// # Errors
	/// Fails if bucket already exists
	pub async fn bucket_create(&self) -> Result {
		BucketProvider::bucket_create(self.provider.as_ref(), &self.name).await
	}

	/// Ensure bucket exists, creating if needed
	pub async fn bucket_try_create(&self) -> Result {
		BucketProvider::bucket_try_create(self.provider.as_ref(), &self.name)
			.await
	}

	/// Check if bucket exists
	pub async fn bucket_exists(&self) -> Result<bool> {
		BucketProvider::bucket_exists(self.provider.as_ref(), &self.name).await
	}

	/// Remove bucket
	///
	/// # Errors
	/// Fails if bucket doesn't exist
	pub async fn bucket_remove(&self) -> Result {
		BucketProvider::bucket_remove(self.provider.as_ref(), &self.name).await
	}

	/// Insert typed object into table
	///
	/// # Example
	/// ```
	/// # use bevy::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let table = temp_table::<TableItem<String>>();
	/// let item = TableItem::new("test data".to_string());
	/// table.push(item).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn push(&self, body: T) -> Result {
		self.provider.insert_row(&self.name, body).await
	}

	/// Insert typed object, failing if it already exists
	///
	/// # Example
	/// ```
	/// # use bevy::prelude::*;
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
	/// Returns error if object already exists at path
	pub async fn try_push(&self, body: T) -> Result {
		let id = body.id();
		if self.exists(body.id()).await? {
			bevybail!("Row already exists: {}", id)
		} else {
			self.push(body).await
		}
	}

	/// Check if object exists at path
	///
	/// # Example
	/// ```
	/// # use bevy::prelude::*;
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
		let path = RoutePath::new(id.to_string());
		BucketProvider::exists(self.provider.as_ref(), &self.name, &path).await
	}

	/// List all object paths in table
	///
	/// # Example
	/// ```
	/// # use bevy::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let table = temp_table::<TableItem<String>>();
	/// let paths = table.list().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn list(&self) -> Result<Vec<RoutePath>> {
		BucketProvider::list(self.provider.as_ref(), &self.name).await
	}

	/// Get typed object data from path
	///
	/// # Example
	/// ```
	/// # use bevy::prelude::*;
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
	/// Returns error if object doesn't exist or fails to deserialize
	pub async fn get(&self, id: Uuid) -> Result<T> {
		self.provider.get_row(&self.name, id).await
	}

	/// Get all objects and their typed data
	///
	/// # Example
	/// ```
	/// # use bevy::prelude::*;
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
	/// Expensive operation - prefer [`Self::list`] + [`Self::get`] for large tables
	pub async fn get_all(&self) -> Result<Vec<(RoutePath, T)>> {
		self.list()
			.await?
			.into_iter()
			.map(async |path| {
				let id = path
					.to_string()
					.trim_start_matches('/')
					.parse::<Uuid>()
					.map_err(|e| {
						bevyhow!("Invalid UUID in path {}: {}", path, e)
					})?;
				let data = self.get(id).await?;
				Ok::<_, BevyError>((path, data))
			})
			.xmap(async_ext::try_join_all)
			.await
	}

	/// Remove object from table at path
	///
	/// # Example
	/// ```
	/// # use bevy::prelude::*;
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
	/// Returns error if object doesn't exist
	pub async fn remove(&self, id: Uuid) -> Result {
		let path = RoutePath::new(id.to_string());
		BucketProvider::remove(self.provider.as_ref(), &self.name, &path).await
	}

	/// Get public URL for object (if supported by provider)
	///
	/// # Example
	/// ```
	/// # use bevy::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let table = temp_table::<TableItem<String>>();
	/// let path = RoutePath::from("/some-uuid");
	/// if let Some(url) = table.public_url(&path).await? {
	///     println!("Public URL: {}", url);
	/// }
	/// # Ok(())
	/// # }
	/// ```
	///
	/// Returns `None` if provider doesn't support public URLs
	pub async fn public_url(&self, path: &RoutePath) -> Result<Option<String>> {
		BucketProvider::public_url(self.provider.as_ref(), &self.name, path)
			.await
	}

	/// Get provider region
	pub async fn region(&self) -> Option<String> {
		BucketProvider::region(self.provider.as_ref())
	}
}


/// Types that can be stored in a [`TableStore`]
///
/// This trait is automatically implemented for any type that implements the required bounds:
/// - [`Serialize`] - For encoding objects into bytes
/// - [`DeserializeOwned`] - For decoding objects from bytes
/// - [`Clone`] - For copying objects
/// - [`'static`] - For type safety across async boundaries
pub trait TableRow: TableContent {
	/// Unique identifier for the object, used as the primary key in the table
	fn id(&self) -> Uuid;
	/// Decodes uuid timestamp as time since unix epoch
	/// ## Panics
	/// Panics if uuid is not v1,v6 or v7
	fn timestamp(&self) -> Duration {
		let timestamp = self.id().get_timestamp().unwrap();
		let (secs, nanos) = timestamp.to_unix();
		Duration::new(secs, nanos)
	}
}
/// Helper blanket trait constraining types which may be included in a table
pub trait TableContent:
	'static + Send + Sync + Clone + Serialize + DeserializeOwned
{
}
impl<T> TableContent for T where
	T: 'static + Send + Sync + Clone + Serialize + DeserializeOwned
{
}


/// Helper type implemementing [`TableData`]. Note some services
/// like dynamodb do not allow indexing nested values, so if thats required
/// a standalone impl [`TableData`] type should be used.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TableItem<T> {
	/// A uuid v7 used as the primary key
	pub id: Uuid,
	/// Duration since Unix epoch
	pub created: SystemTime,
	pub data: T,
}

impl<T> TableItem<T> {
	pub fn new(data: T) -> Self {
		Self {
			id: Uuid::now_v7(),
			created: SystemTime::now(),
			data,
		}
	}
}
impl<T: TableContent> TableRow for TableItem<T> {
	fn id(&self) -> Uuid { self.id }
}


/// Storage provider for typed table operations
///
/// This trait extends [`BucketProvider`] with type-safe operations for storing
/// and retrieving serializable objects. Implementors only need to provide
/// [`box_clone_table`], as the default implementations just use the [`BucketProvider`] api
/// to store the data as json.
pub trait TableProvider<T: TableRow>:
	BucketProvider + 'static + Send + Sync
{
	fn box_clone_table(&self) -> Box<dyn TableProvider<T>>;
	fn insert_row(
		&self,
		bucket_name: &str,
		body: T,
	) -> SendBoxedFuture<Result> {
		let path = RoutePath::new(body.id().to_string());
		match serde_json::to_vec(&body) {
			Ok(vec) => {
				BucketProvider::insert(self, bucket_name, &path, vec.into())
			}
			Err(e) => {
				Box::pin(async move { bevybail!("Failed to serialize: {}", e) })
			}
		}
	}
	fn get_row(
		&self,
		bucket_name: &str,
		id: Uuid,
	) -> SendBoxedFuture<Result<T>> {
		let path = RoutePath::new(id.to_string());
		let fut = BucketProvider::get(self, bucket_name, &path);
		Box::pin(async move {
			let bytes = fut.await?;
			match serde_json::from_slice(&bytes) {
				Ok(val) => Ok(val),
				Err(e) => bevybail!("Failed to deserialize: {}", e),
			}
		})
	}
}

/// Create temporary in-memory bucket for testing
pub fn temp_table<T: TableRow>() -> TableStore<T> {
	TableStore::new(InMemoryProvider::new(), "temp")
}

/// Select filesystem or DynamoDb TableProvider based on [`ServiceAccess`] and feature flags
#[allow(unused_variables)]
pub async fn dynamo_fs_selector<T: TableRow>(
	fs_path: &AbsPathBuf,
	table_name: &str,
	access: ServiceAccess,
) -> TableStore<T> {
	match access {
		ServiceAccess::Local => {
			debug!("Table Selector - FS: {fs_path}");
			TableStore::new(FsBucketProvider::new(fs_path.clone()), "")
		}
		#[cfg(not(all(feature = "aws", not(target_arch = "wasm32"))))]
		ServiceAccess::Remote => {
			debug!("Table Selector - FS (no aws feature): {fs_path}");
			TableStore::new(FsBucketProvider::new(fs_path.clone()), "")
		}
		#[cfg(all(feature = "aws", not(target_arch = "wasm32")))]
		ServiceAccess::Remote => {
			debug!("Table Selector - Dynamo: {table_name}");
			let provider = DynamoDbProvider::create().await;
			TableStore::new(provider, table_name)
		}
	}
}

#[cfg(test)]
pub mod table_test {
	use crate::prelude::*;
	use serde::Deserialize;
	use serde::Serialize;
	use sweet::prelude::*;

	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
	pub struct MyObject {
		some_key: String,
		some_vec: Vec<MyObject>,
	}

	pub async fn run(provider: impl TableProvider<TableItem<MyObject>>) {
		let table = TableStore::new(provider, "beet-test-table");
		let body = TableItem::new(MyObject {
			some_key: "some_value".into(),
			some_vec: vec![MyObject {
				some_key: "nested".into(),
				some_vec: vec![],
			}],
		});
		let id = body.id();
		let path = RoutePath::new(id.to_string());
		table.bucket_remove().await.ok();
		table.bucket_exists().await.unwrap().xpect_false();
		table.bucket_try_create().await.unwrap();
		table.exists(id).await.unwrap().xpect_false();
		table.remove(id).await.xpect_err();
		table.push(body.clone()).await.unwrap();
		table.bucket_exists().await.unwrap().xpect_true();
		table.exists(id).await.unwrap().xpect_true();
		table.list().await.unwrap().xpect_eq(vec![path.clone()]);
		table.get(id).await.unwrap().xpect_eq(body.clone());
		table.get(id).await.unwrap().xpect_eq(body);

		table.remove(id).await.unwrap();
		table.get(id).await.xpect_err();

		table.bucket_remove().await.unwrap();
		table.bucket_exists().await.unwrap().xpect_false();
	}
}
