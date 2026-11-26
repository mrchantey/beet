use crate::prelude::*;
use alloc::sync::Arc;
use beet_core::prelude::bevy_ecs::error::ErrorContext;
use beet_core::prelude::*;
use core::marker::PhantomData;
use heck::ToSnakeCase;
use serde::Deserialize;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Type-erased handle to one partition's tables, the table twin of
/// [`BlobStore`]: wraps an [`Arc<dyn TableProvider>`] and is materialized onto
/// the entity of a table store component ([`SqliteStore`](crate::prelude::SqliteStore),
/// `DynamoStore`, [`DurableObjectStore`], a [`JsonTableStore`]) by that
/// component, so a consumer resolves `TableStore` from an entity and never
/// names a backend.
///
/// A table store is declared. No blob store is one, and a consumer finding
/// none is refused naming what asked and how to declare one
/// ([`missing`](Self::missing)), never given a fallback: resolve with
/// [`nearest`](Self::nearest) or, for the target of a [`StoreRef`],
/// [`declared`](Self::declared).
///
/// Typed access goes through [`Self::table`]; the erased methods here take
/// [`StoredRow`]s. A table named by an NSID is a record table, and every
/// write to one is validated here as a record of that collection, so the
/// erased path cannot store an invalid record either (a snake case table name
/// never parses as an NSID, so the rule costs nothing elsewhere).
#[derive(Clone, Component)]
pub struct TableStore {
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

	/// A store over an already erased provider.
	pub fn from_box(provider: Box<dyn TableProvider>) -> Self {
		Self {
			provider: Arc::from(provider),
		}
	}

	/// The table store a [`StoreUri`] names, for a caller with no entity to
	/// land the concrete component on, see [`TableStoreProvider::from_uri`].
	#[cfg(feature = "std")]
	pub fn from_uri(uri: &StoreUri) -> Result<Self> {
		TableStoreProvider::from_uri(uri)?.into_table_store().xok()
	}

	/// The table store nearest `entity`, its own or an ancestor's, else the
	/// error naming `asker`, ie `` `<ServeTables/>` ``.
	pub async fn nearest(
		entity: &AsyncEntity,
		asker: impl core::fmt::Display,
	) -> Result<Self> {
		entity
			.with_state::<AncestorQuery<&TableStore>, _>(|entity, stores| {
				stores.get(entity).ok().cloned()
			})
			.await?
			.ok_or_else(|| Self::missing(asker))
	}

	/// The table store `target` itself declares, the one a [`StoreRef`] names,
	/// waiting for a declaration still landing, else the error naming
	/// `asker`.
	#[cfg(feature = "std")]
	pub async fn declared(
		world: &AsyncWorld,
		target: Entity,
		asker: impl core::fmt::Display,
	) -> Result<Self> {
		StoreRef::find::<Self>(world, target)
			.await
			.ok_or_else(|| Self::missing(asker))
	}

	/// The error of `asker` needing a table store where there is none.
	pub fn missing(asker: impl core::fmt::Display) -> BevyError {
		bevyhow!(
			"{asker} needs a table store and none is declared for it: a blob \
			 store is not one. Declare `<SqliteStore path=\"..\"/>` (the \
			 `sqlite` feature), a `<DynamoTableBlock/>` or a \
			 `<DurableObjectStore/>`, or to keep rows as json files in a blob \
			 store opt into `JsonTableStore`, ie \
			 `<FsStore path=\"..\" {{JsonTableStore}}/>`"
		)
	}

	/// Land this as the entity's table store. An entity has one, so another
	/// backend's already there is an error naming both, never replaced.
	pub(crate) fn land(self, entity: &mut EntityWorldMut) {
		if let Some(existing) = entity.get::<TableStore>()
			&& existing.provider.id() != self.provider.id()
		{
			let err = bevyhow!(
				"two table stores are declared on one entity, `{}` and `{}`: \
				 declare each on its own",
				existing.describe(),
				self.describe()
			);
			entity.world_scope(|world| {
				world.handle_command_error::<TableStore>(err)
			});
			return;
		}
		entity.insert(self);
	}

	/// The provider, for a caller serving it on (the wire server).
	pub fn provider(&self) -> &dyn TableProvider { self.provider.as_ref() }

	/// A typed view over one table of this store.
	pub fn table<T: TableStoreRow>(&self) -> Table<T> {
		Table {
			store: self.clone(),
			spec: TableSpec::of::<T>(),
			_marker: PhantomData,
		}
	}

	/// The partition `id` of this declaration, its own database.
	pub fn partition(&self, id: impl AsRef<str>) -> Result<Self> {
		let id = PartitionId::parse(id.as_ref())?;
		Self::from_box(self.provider.partition(&id)).xok()
	}

	/// Whether this backend supports `capability`.
	pub fn supports(&self, capability: Capability) -> bool {
		self.provider.supports(capability)
	}

	/// Whether `event` is of a blob these tables are kept in, see
	/// [`TableProvider::holds_blob`].
	pub fn holds_blob(&self, event: &BlobEvent) -> bool {
		self.provider.holds_blob(event)
	}

	/// Where this store lives, ie `dynamo:beet-site--prod--analytics
	/// (us-west-2)`, for any error naming it.
	pub fn describe(&self) -> String { self.provider.describe() }

	/// The row at `key` in `table`.
	pub async fn get(
		&self,
		table: impl Into<TableSpec>,
		key: impl Into<TableKey>,
	) -> Result<Option<StoredRow>> {
		self.provider.get_row(&table.into(), &key.into()).await
	}

	/// Write `row` under `precondition`, validated as a record when `table`
	/// is a record table.
	pub async fn put(
		&self,
		table: impl Into<TableSpec>,
		row: StoredRow,
		precondition: Precondition,
	) -> Result {
		let table = table.into();
		Self::validate(&table, &row)?;
		self.provider.put_row(&table, row, precondition).await
	}

	/// Remove the row at `key` under `precondition`.
	pub async fn remove(
		&self,
		table: impl Into<TableSpec>,
		key: impl Into<TableKey>,
		precondition: Precondition,
	) -> Result {
		self.provider
			.remove_row(&table.into(), &key.into(), precondition)
			.await
	}

	/// One page of `table`.
	pub async fn scan(
		&self,
		table: impl Into<TableSpec>,
		scan: Scan,
	) -> Result<ScanPage> {
		self.provider.scan_rows(&table.into(), scan).await
	}

	/// Apply every write or none, each validated as [`put`](Self::put)
	/// validates.
	pub async fn apply(&self, writes: Vec<TableWrite>) -> Result {
		for write in &writes {
			if let TableWrite::Put { table, row, .. } = write {
				Self::validate(table, row)?;
			}
		}
		self.provider.apply(writes).await
	}

	/// Retire the rows at `keys` in `table`, see
	/// [`TableProvider::expire_rows`].
	pub async fn expire(
		&self,
		table: impl Into<TableSpec>,
		keys: Vec<TableKey>,
	) -> Result {
		self.provider.expire_rows(&table.into(), keys).await
	}

	/// One page of `table` in the order of its declared index `index`.
	pub async fn scan_index(
		&self,
		table: impl Into<TableSpec>,
		index: &str,
		scan: IndexScan,
	) -> Result<ScanPage> {
		let table = table.into();
		let index = table.index(index)?.clone();
		self.provider.scan_index(&table, &index, scan).await
	}

	/// Create the backing store (may take 10+ seconds for cloud providers).
	pub async fn store_create(&self) -> Result {
		self.provider.store_create().await
	}
	/// Ensure the backing store exists, creating it if needed.
	pub async fn store_try_create(&self) -> Result {
		self.provider.store_try_create().await
	}
	/// Whether the backing store exists.
	pub async fn store_exists(&self) -> Result<bool> {
		self.provider.store_exists().await
	}
	/// Remove the backing store.
	pub async fn store_remove(&self) -> Result {
		self.provider.store_remove().await
	}

	/// Refuse a write a backend must not store: a row whose key is
	/// invalid, and in a record table anything that is not a record of its
	/// collection at an rkey.
	fn validate(table: &TableSpec, row: &StoredRow) -> Result {
		row.key().validate()?;
		let Some(collection) = table.collection() else {
			return Ok(());
		};
		Rkey::parse(row.key()).map_err(|err| {
			bevyhow!("record `{collection}/{}`: {err}", row.key())
		})?;
		let record =
			AtprotoValue::from_wire(row.row().clone().into_json_form())
				.map_err(|err| {
					bevyhow!("record `{collection}/{}`: {err}", row.key())
				})?;
		match record.as_map().ok().and_then(|map| map.get("$type").ok()) {
			Some(Value::Str(r#type))
				if r#type.as_str() == collection.as_str() =>
			{
				Ok(())
			}
			Some(other) => bevybail!(
				"record `{collection}/{}` declares `$type: {other}`",
				row.key()
			),
			None => bevybail!(
				"record `{collection}/{}` carries no `$type`",
				row.key()
			),
		}
	}

	/// Component hook that reads a concrete table provider component from the
	/// entity and inserts a [`TableStore`] wrapping it.
	/// Use with `#[component(on_insert = TableStore::on_insert::<MyStore>)]`.
	pub fn on_insert<T: Component + Clone + TableProvider>(
		mut world: DeferredWorld,
		cx: HookContext,
	) {
		match world.entity(cx.entity).get_or_else::<T>().cloned() {
			Ok(provider) => {
				world.commands().entity(cx.entity).queue_silenced(
					move |mut entity: EntityWorldMut| {
						TableStore::new(provider).land(&mut entity)
					},
				);
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

/// Typed view over one table of a [`TableStore`], rows converted through
/// [`TableStoreRow::into_row`] and [`TableStoreRow::from_row`] at this edge.
/// The table twin of [`Blob`].
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// # async fn run() -> Result<()> {
/// let table = Table::<TableItem<u32>>::new(InMemoryTableStore::new());
/// let item = TableItem::new(7);
/// let cid = table.push(item.clone()).await?;
/// // read, modify, write back only if nothing else wrote meanwhile
/// let (mut row, cid) = table.get_with_cid(item.key()).await?.unwrap();
/// row.data += 1;
/// table.put_if(row, Precondition::Matches(cid)).await?;
/// # Ok(())
/// # }
/// ```
pub struct Table<T: TableStoreRow> {
	store: TableStore,
	spec: TableSpec,
	_marker: PhantomData<T>,
}

impl<T: TableStoreRow> Clone for Table<T> {
	fn clone(&self) -> Self {
		Self {
			store: self.store.clone(),
			spec: self.spec.clone(),
			_marker: PhantomData,
		}
	}
}

impl<T: TableStoreRow> Table<T> {
	/// The table over `provider`.
	pub fn new(provider: impl TableProvider) -> Self {
		TableStore::new(provider).table()
	}

	/// The table's name, [`TableStoreRow::table_name`].
	pub fn name(&self) -> &str { &self.spec.name }

	/// The table as a provider sees it.
	pub fn spec(&self) -> &TableSpec { &self.spec }

	/// The store this table lives in.
	pub fn store(&self) -> &TableStore { &self.store }

	/// Create the backing store (may take 10+ seconds for cloud providers).
	pub async fn store_create(&self) -> Result {
		self.store.store_create().await
	}
	/// Ensure the backing store exists, creating it if needed.
	pub async fn store_try_create(&self) -> Result {
		self.store.store_try_create().await
	}
	/// Whether the backing store exists.
	pub async fn store_exists(&self) -> Result<bool> {
		self.store.store_exists().await
	}
	/// Remove the backing store.
	pub async fn store_remove(&self) -> Result {
		self.store.store_remove().await
	}

	/// The stored row a typed row becomes.
	pub fn stored(&self, value: T) -> Result<StoredRow> {
		let key = value.key();
		StoredRow::new(key, value.into_row()?)
	}

	/// The row at `key`, `None` where there is none.
	pub async fn get(&self, key: impl Into<TableKey>) -> Result<Option<T>> {
		self.get_with_cid(key).await?.map(|(value, _)| value).xok()
	}

	/// The row at `key` with its cid, what a conditional write swaps on.
	pub async fn get_with_cid(
		&self,
		key: impl Into<TableKey>,
	) -> Result<Option<(T, Cid)>> {
		match self.store.get(&self.spec, key).await? {
			Some(row) => {
				let (key, cid, row) = row.into_parts();
				Some((T::from_row(&key, row)?, cid))
			}
			None => None,
		}
		.xok()
	}

	/// Whether a row exists at `key`.
	pub async fn exists(&self, key: impl Into<TableKey>) -> Result<bool> {
		self.store.get(&self.spec, key).await?.is_some().xok()
	}

	/// Write `value` at its key whatever is there, answering its cid.
	pub async fn put(&self, value: T) -> Result<Cid> {
		self.put_if(value, Precondition::Any).await
	}

	/// Write `value` at its key, failing with [`TableError::Conflict`] if a
	/// row is already there.
	pub async fn push(&self, value: T) -> Result<Cid> {
		self.put_if(value, Precondition::Absent).await
	}

	/// Write `value` at its key when `precondition` holds, answering its
	/// cid.
	pub async fn put_if(
		&self,
		value: T,
		precondition: Precondition,
	) -> Result<Cid> {
		let row = self.stored(value)?;
		let cid = row.cid().clone();
		self.store.put(&self.spec, row, precondition).await?;
		cid.xok()
	}

	/// Remove the row at `key`, if any.
	pub async fn remove(&self, key: impl Into<TableKey>) -> Result {
		self.remove_if(key, Precondition::Any).await
	}

	/// Remove the row at `key` when `precondition` holds.
	pub async fn remove_if(
		&self,
		key: impl Into<TableKey>,
		precondition: Precondition,
	) -> Result {
		self.store.remove(&self.spec, key, precondition).await
	}

	/// The write putting `value` under `precondition`, for
	/// [`TableStore::apply`].
	pub fn put_write(
		&self,
		value: T,
		precondition: Precondition,
	) -> Result<TableWrite> {
		TableWrite::Put {
			table: self.spec.clone(),
			row: self.stored(value)?,
			precondition,
		}
		.xok()
	}

	/// The write removing `key` under `precondition`, for
	/// [`TableStore::apply`].
	pub fn remove_write(
		&self,
		key: impl Into<TableKey>,
		precondition: Precondition,
	) -> TableWrite {
		TableWrite::Remove {
			table: self.spec.clone(),
			key: key.into(),
			precondition,
		}
	}

	/// One page of rows in key order.
	pub async fn scan(&self, scan: Scan) -> Result<TablePage<T>> {
		self.store
			.scan(&self.spec, scan)
			.await?
			.try_map(Self::decode)
	}

	/// One page of rows in the order of the declared index `index`.
	///
	/// As consistent as the backend's index: immediately on SQLite, a
	/// Durable Object and the in-process host, eventually on a Dynamo global
	/// secondary index, which may briefly answer a row's previous position.
	pub async fn scan_index(
		&self,
		index: &str,
		scan: IndexScan,
	) -> Result<TablePage<T>> {
		self.store
			.scan_index(&self.spec, index, scan)
			.await?
			.try_map(Self::decode)
	}

	/// Every row within `scan`'s bounds, paging until the last.
	pub async fn scan_all(&self, scan: Scan) -> Result<Vec<T>> {
		self.scan_stored(scan)
			.await?
			.into_iter()
			.map(Self::decode)
			.collect()
	}

	/// Every stored row within `scan`'s bounds, paging until the last.
	pub async fn scan_stored(&self, mut scan: Scan) -> Result<Vec<StoredRow>> {
		let mut rows = Vec::new();
		loop {
			let page = self.store.scan(&self.spec, scan.clone()).await?;
			rows.extend(page.rows);
			match page.cursor {
				Some(cursor) => scan.cursor = Some(cursor),
				None => break,
			}
		}
		rows.xok()
	}

	/// Every key in the table, in order.
	pub async fn list(&self) -> Result<Vec<TableKey>> {
		self.scan_stored(Scan::default())
			.await?
			.into_iter()
			.map(|row| row.into_parts().0)
			.collect::<Vec<_>>()
			.xok()
	}

	/// Every row in the table, in key order.
	pub async fn get_all(&self) -> Result<Vec<T>> {
		self.scan_all(Scan::default()).await
	}

	/// Like [`Self::get_all`], but a row that fails to decode (eg a legacy
	/// schema) is skipped with a warning instead of failing the whole read,
	/// so a caller reporting aggregates over it should not present the count
	/// as the table's true total.
	pub async fn get_all_lossy(&self) -> Result<Vec<T>> {
		self.scan_stored(Scan::default())
			.await?
			.into_iter()
			.filter_map(|row| {
				let key = row.key().clone();
				match Self::decode(row) {
					Ok(row) => Some(row),
					Err(err) => {
						warn!(
							"skipping unreadable row {}/{key}: {err}",
							self.spec.name
						);
						None
					}
				}
			})
			.collect::<Vec<_>>()
			.xok()
	}

	/// Where this table lives, for any error naming it.
	pub fn describe(&self) -> String { self.store.describe() }

	fn decode(row: StoredRow) -> Result<T> {
		let (key, _, row) = row.into_parts();
		T::from_row(&key, row)
	}
}

/// Types stored as rows of a [`Table`]: the table they live in, the key each
/// row is addressed by, the indexes the table keeps, and the conversion to
/// and from a [`Row`], by default through serde.
pub trait TableStoreRow: TableContent {
	/// The table this row type lives in, ie `youtube_likes`: by default the
	/// snake case of the short type name, generics dropped, so `TableItem<u32>`
	/// lives in `table_item`.
	fn table_name() -> SmolStr {
		let name = TypeName::of::<Self>().short();
		name.split_once('<')
			.map_or(name.as_str(), |(base, _generics)| base)
			.to_snake_case()
			.into()
	}
	/// The row's primary key.
	fn key(&self) -> TableKey;
	/// The secondary indexes the table keeps, none by default.
	fn indexes() -> Vec<TableIndex> { Vec::new() }
	/// This value as a row.
	fn into_row(self) -> Result<Row> { Row::from_serde(&self) }
	/// The value a row at `key` holds.
	fn from_row(key: &TableKey, row: Row) -> Result<Self> {
		let _ = key;
		row.into_serde()
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

/// A record table: the table is the collection, the key the rkey, and the
/// row the record in the data model with its `$type` written, so a float is
/// the `org.beet.core#float` object it crosses into a repo as and the row's
/// cid is the record's.
impl<T> TableStoreRow for Rkeyed<T>
where
	T: TableContent + AtprotoRecord,
{
	fn table_name() -> SmolStr { T::COLLECTION.as_str().into() }
	fn key(&self) -> TableKey { self.rkey().into() }
	fn into_row(self) -> Result<Row> {
		let record =
			AtprotoValue::from_serde(&*self)?.into_record(&T::COLLECTION)?;
		Row::from_value((*record).clone())
	}
	fn from_row(key: &TableKey, row: Row) -> Result<Self> {
		let record = AtprotoValue::from_wire(row.into_json_form())?;
		Rkeyed::new(Rkey::parse(key)?, record.into_serde::<T>()?).xok()
	}
}

/// A row of data with a TID key minted when it is made, so its table scans
/// in creation order. Some services do not index nested values (DynamoDB),
/// so a row whose fields must be indexed implements [`TableStoreRow`] itself.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TableItem<T> {
	/// The primary key, minted from the clock.
	pub id: Tid,
	/// Wall-clock creation time.
	pub created: Timestamp,
	/// The user-provided data payload.
	pub data: T,
}

impl<T> TableItem<T> {
	/// A new item keyed by a fresh TID.
	pub fn new(data: T) -> Self {
		Self {
			id: Tid::now(),
			created: Timestamp::now(),
			data,
		}
	}
}

impl<T: TableContent> TableStoreRow for TableItem<T> {
	fn key(&self) -> TableKey { self.id.into() }
}
