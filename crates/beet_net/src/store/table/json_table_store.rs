//! Tables as json files in a [`BlobStore`], the naive one: see
//! [`JsonTableStore`].
use crate::prelude::*;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use beet_core::prelude::*;
use bevy::platform::sync::Mutex;
use bytes::Bytes;

/// Tables over the nearest [`BlobStore`] at or above this entity, one json
/// file per partition: for trying things in a small binary and in a browser
/// tab, where a database is more than the job needs. Opted into by name,
/// since a blob store alone is never a table store:
///
/// ```html
/// <FsStore path="target/chat" {JsonTableStore}/>
/// <LocalStorageStore store_name="chat" {JsonTableStore{path:"tables"}}/>
/// ```
///
/// The [`TableStore`] over it lands beside it once the blob store does, and
/// follows it as stores are inserted and removed above. In rust, any blob
/// store value is one through [`JsonTableStore::over`].
///
/// - **The file**: `@root.json` for the declaration's own partition and
///   `<partition>.json` for any other, pretty json with sorted keys, every
///   table of the partition as `{table: {key: row}}`. Rows are as a person
///   would write them, a byte string `{"$bytes": "<base64>"}`, so a file may
///   be read, diffed and edited by hand; a row's cid is computed as it is
///   read.
/// - **A write costs the size of its partition**: every operation reads the
///   whole file and a write replaces it, so a table that grows belongs in a
///   [`SqliteStore`](crate::prelude::SqliteStore) or a deployed database.
/// - **A write is atomic**: the file is replaced only while it is the one
///   that was read ([`BlobCondition::Matches`]), and read again when it is
///   not, so only a row's own [`Precondition`] fails a write. Batches are
///   atomic and an index is a sort, both within the partition.
/// - **Its files are not content**: a watched repo store it sits in reloads
///   nothing on a write to one.
/// - **It needs conditional writes**, so a blob store without them (one read
///   over http, a [`StoreFork`]) is refused.
/// - **As safe as those writes are**: across processes on a filesystem (see
///   [`FsStore`]'s docs) and in a bucket, and within one tab in a browser's
///   storage, where two tabs writing one partition may erase each other's
///   rows.
/// - **Its lifecycle is the blob store's**: it is created with it, and
///   [`store_remove`](StoreBackend::store_remove) removes the partition files
///   alone, refused where the blob store holds anything else below `path`.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, Get, SetWith, Component, Reflect,
)]
#[reflect(Component, Default)]
pub struct JsonTableStore {
	/// The directory of the blob store the files are in, its root by default.
	#[set_with(unwrap_option, into)]
	path: Option<RelPath>,
}

impl JsonTableStore {
	/// The tables in `blobs`, landing nothing anywhere.
	///
	/// # Errors
	/// Errors where `blobs` has no conditional writes.
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let tables = JsonTableStore::over(BlobStore::temp())?;
	/// tables.table::<TableItem<u32>>().push(TableItem::new(7)).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub fn over(blobs: BlobStore) -> Result<TableStore> {
		JsonTableProvider::new(blobs)?.xmap(TableStore::new).xok()
	}

	/// The tables this declares in `blobs`.
	pub fn tables(&self, blobs: &BlobStore) -> Result<TableStore> {
		match &self.path {
			Some(path) => blobs.with_subdir(path.clone()),
			None => blobs.clone(),
		}
		.xmap(Self::over)
	}
}

/// The [`TableProvider`] a [`JsonTableStore`] declares.
#[derive(Debug, Clone)]
pub struct JsonTableProvider {
	blobs: BlobStore,
	partition: Option<PartitionId>,
	/// The files read or written through any partition of this declaration,
	/// so a watcher tells them from the blobs beside them.
	opened: Arc<Mutex<HashSet<RelPath>>>,
}

impl JsonTableProvider {
	/// The file of the declaration's own partition, whose `@` no partition id
	/// holds.
	const ROOT: &'static str = "@root";
	/// The extension of every partition's file.
	const EXTENSION: &'static str = ".json";

	/// The provider of the tables in `blobs`.
	///
	/// # Errors
	/// Errors where `blobs` has no conditional writes.
	pub fn new(blobs: BlobStore) -> Result<Self> {
		if !blobs.conditional_writes() {
			bevybail!(
				"a `JsonTableStore` needs a blob store with conditional \
				 writes, which `{}` has none of: declare it over another \
				 one, ie `<FsStore path=\"..\" {{JsonTableStore}}/>`, or a \
				 table store of its own, ie `<SqliteStore path=\"..\"/>`",
				blobs.describe()
			);
		}
		Self {
			blobs,
			partition: None,
			opened: default(),
		}
		.xok()
	}

	/// The file this partition is.
	fn file(&self) -> RelPath {
		let name = self
			.partition
			.as_ref()
			.map_or(Self::ROOT, PartitionId::as_str);
		RelPath::new(format!("{name}{}", Self::EXTENSION))
	}

	/// Whether `path` is named as a partition's file.
	fn is_partition(path: &RelPath) -> bool {
		path.as_str()
			.strip_suffix(Self::EXTENSION)
			.is_some_and(|name| {
				name == Self::ROOT || PartitionId::parse(name).is_ok()
			})
	}

	/// The partition as it is now, and the version it was read at, `None` for
	/// a partition never written.
	async fn read(&self) -> Result<(PartitionRows, Option<BlobVersion>)> {
		let file = self.file();
		self.opened
			.lock()
			.unwrap_or_else(|err| err.into_inner())
			.insert(file.clone());
		match self.blobs.get_versioned(&file).await? {
			Some((bytes, version)) => (
				PartitionRows::parse_json(&bytes).map_err(|err| {
					bevyhow!(
						"`{file}` in `{}` is not a table partition: {err}",
						self.blobs.describe()
					)
				})?,
				Some(version),
			),
			None => (default(), None),
		}
		.xok()
	}

	/// Change the partition with `func`, which answers whether it did, and
	/// replace the file only while it is the one `func` was given: another
	/// writer's change in between reads it again.
	async fn update(
		&self,
		func: impl Fn(&mut PartitionRows) -> Result<bool>,
	) -> Result {
		let file = self.file();
		loop {
			let (mut partition, version) = self.read().await?;
			if !func(&mut partition)? {
				return Ok(());
			}
			let condition = match version {
				Some(version) => BlobCondition::Matches(version),
				None => BlobCondition::Absent,
			};
			let written = match partition.is_empty() {
				true => self.blobs.remove_if(&file, condition).await,
				false => self
					.blobs
					.insert_if(&file, partition.to_json_bytes(), condition)
					.await
					.map(drop),
			};
			match written {
				Err(err) if err.downcast_ref::<BlobConflict>().is_some() => {
					async_ext::yield_now().await;
				}
				written => return written,
			}
		}
	}
}

impl StoreBackend for JsonTableProvider {
	fn id(&self) -> &'static str { "json" }

	fn describe(&self) -> String {
		format!("json:{}/{}", self.blobs.describe(), self.file())
	}

	fn region(&self) -> Option<String> { self.blobs.region() }

	fn store_exists(&self) -> SendBoxedFuture<Result<bool>> {
		self.blobs.store_exists()
	}

	fn store_create(&self) -> SendBoxedFuture<Result> {
		self.blobs.store_create()
	}

	fn store_remove(&self) -> SendBoxedFuture<Result> {
		let blobs = self.blobs.clone();
		Box::pin(async move {
			let files = blobs.list().await?;
			for file in &files {
				let partition = match Self::is_partition(file) {
					true => PartitionRows::parse_json(&blobs.get(file).await?)
						.is_ok(),
					false => false,
				};
				if !partition {
					bevybail!(
						"refusing to remove the json tables in `{}`: it holds \
						 `{file}`, which is not one of them. Give the \
						 `JsonTableStore` a `path` of its own, or remove the \
						 blob store itself",
						blobs.describe()
					);
				}
			}
			for file in &files {
				blobs.remove(file).await?;
			}
			Ok(())
		})
	}
}

impl TableProvider for JsonTableProvider {
	fn box_clone_table(&self) -> Box<dyn TableProvider> {
		Box::new(self.clone())
	}

	fn partition(&self, id: &PartitionId) -> Box<dyn TableProvider> {
		Box::new(Self {
			blobs: self.blobs.clone(),
			opened: self.opened.clone(),
			partition: Some(match &self.partition {
				Some(parent) => parent.join(id),
				None => id.clone(),
			}),
		})
	}

	fn supports(&self, capability: Capability) -> bool {
		matches!(capability, Capability::AtomicApply | Capability::Indexes)
	}

	fn holds_blob(&self, event: &BlobEvent) -> bool {
		self.opened
			.lock()
			.unwrap_or_else(|err| err.into_inner())
			.iter()
			.any(|file| self.blobs.matches_object(event, file))
	}

	fn get_row(
		&self,
		table: &TableSpec,
		key: &TableKey,
	) -> SendBoxedFuture<Result<Option<StoredRow>>> {
		let this = self.clone();
		let table = table.clone();
		let key = key.clone();
		Box::pin(async move { this.read().await?.0.get(&table, &key) })
	}

	fn put_row(
		&self,
		table: &TableSpec,
		row: StoredRow,
		precondition: Precondition,
	) -> SendBoxedFuture<Result> {
		self.apply(vec![TableWrite::Put {
			table: table.clone(),
			row,
			precondition,
		}])
	}

	fn remove_row(
		&self,
		table: &TableSpec,
		key: &TableKey,
		precondition: Precondition,
	) -> SendBoxedFuture<Result> {
		self.apply(vec![TableWrite::Remove {
			table: table.clone(),
			key: key.clone(),
			precondition,
		}])
	}

	fn scan_rows(
		&self,
		table: &TableSpec,
		scan: Scan,
	) -> SendBoxedFuture<Result<ScanPage>> {
		let this = self.clone();
		let table = table.clone();
		Box::pin(async move { this.read().await?.0.scan(&table, &scan) })
	}

	fn apply(&self, writes: Vec<TableWrite>) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move {
			this.update(|partition| {
				let mut changed = false;
				for write in &writes {
					changed |= partition.apply(write)?;
				}
				changed.xok()
			})
			.await
		})
	}

	fn expire_rows(
		&self,
		table: &TableSpec,
		keys: Vec<TableKey>,
	) -> SendBoxedFuture<Result> {
		self.apply(
			keys.into_iter()
				.map(|key| TableWrite::Remove {
					table: table.clone(),
					key,
					precondition: Precondition::Any,
				})
				.collect(),
		)
	}

	fn scan_index(
		&self,
		table: &TableSpec,
		index: &TableIndex,
		scan: IndexScan,
	) -> SendBoxedFuture<Result<ScanPage>> {
		let this = self.clone();
		let table = table.clone();
		let index = index.clone();
		Box::pin(async move {
			this.read().await?.0.scan_index(&table, &index, &scan)
		})
	}
}

/// A partition as its json file.
impl PartitionRows {
	/// The partition a file holds.
	fn parse_json(bytes: &[u8]) -> Result<Self> {
		let Value::Map(tables) =
			Value::parse_json(core::str::from_utf8(bytes)?)?
		else {
			bevybail!("expected a map of tables");
		};
		let tables = tables
			.into_iter()
			.map(|(name, rows)| {
				let Value::Map(rows) = rows else {
					bevybail!("table `{name}` is not a map of rows");
				};
				let rows = rows
					.into_iter()
					.map(|(key, row)| {
						let row = Row::from_json_form(row).map_err(|err| {
							bevyhow!("row `{name}/{key}`: {err}")
						})?;
						let key = TableKey::from(key);
						key.validate()?;
						(key, row).xok()
					})
					.collect::<Result<BTreeMap<_, _>>>()?;
				(name, rows).xok()
			})
			.collect::<Result<BTreeMap<_, _>>>()?;
		Self(tables).xok()
	}

	/// The file this partition is written as.
	fn to_json_bytes(&self) -> Bytes {
		let mut text = self
			.0
			.iter()
			.map(|(name, rows)| {
				let rows = rows
					.iter()
					.map(|(key, row)| {
						(
							SmolStr::from(key.as_str()),
							row.clone().into_json_form(),
						)
					})
					.collect::<Map>();
				(name.clone(), Value::Map(rows))
			})
			.collect::<Map>()
			.xmap(Value::Map)
			.to_json_pretty();
		text.push('\n');
		Bytes::from(text)
	}
}

/// Land the [`TableStore`] of each [`JsonTableStore`] in `entities` over its
/// nearest blob store, its own included.
fn resolve_json_tables(
	entities: impl IntoIterator<Item = Entity>,
	tables: &Query<&JsonTableStore>,
	stores: &AncestorQuery<&BlobStore>,
	commands: &mut Commands,
) {
	for entity in entities {
		if let Ok(declared) = tables.get(entity)
			&& let Ok(blobs) = stores.get(entity)
		{
			let (declared, blobs) = (declared.clone(), blobs.clone());
			commands.entity(entity).queue_silenced(
				move |mut entity: EntityWorldMut| match declared.tables(&blobs)
				{
					Ok(tables) => tables.land(&mut entity),
					Err(err) => entity.world_scope(|world| {
						world.handle_command_error::<JsonTableStore>(err)
					}),
				},
			);
		}
	}
}

/// A declaration lands its tables.
pub(crate) fn on_insert_json_tables(
	ev: On<Insert<JsonTableStore>>,
	tables: Query<&JsonTableStore>,
	stores: AncestorQuery<&BlobStore>,
	mut commands: Commands,
) {
	resolve_json_tables([ev.entity], &tables, &stores, &mut commands);
}

/// A blob store inserted at or above a declaration is the one it is now over.
pub(crate) fn on_insert_json_table_blobs(
	ev: On<Insert<BlobStore>>,
	children: Query<&Children>,
	tables: Query<&JsonTableStore>,
	stores: AncestorQuery<&BlobStore>,
	mut commands: Commands,
) {
	resolve_json_tables(
		children.iter_descendants_inclusive(ev.entity),
		&tables,
		&stores,
		&mut commands,
	);
}

/// A declaration parented under a blob store after it spawned.
pub(crate) fn on_parent_json_tables(
	ev: On<Insert<ChildOf>>,
	children: Query<&Children>,
	tables: Query<&JsonTableStore>,
	stores: AncestorQuery<&BlobStore>,
	mut commands: Commands,
) {
	resolve_json_tables(
		children.iter_descendants_inclusive(ev.entity),
		&tables,
		&stores,
		&mut commands,
	);
}

/// A removed blob store takes the tables over it with it.
pub(crate) fn on_remove_json_table_blobs(
	ev: On<Remove<BlobStore>>,
	children: Query<&Children>,
	tables: Query<&JsonTableStore>,
	stores: AncestorQuery<(Entity, &BlobStore)>,
	mut commands: Commands,
) {
	for entity in children.iter_descendants_inclusive(ev.entity) {
		// the store is still present during this observer
		if tables.contains(entity)
			&& stores
				.get(entity)
				.is_ok_and(|(holder, _)| holder == ev.entity)
		{
			commands.entity(entity).try_remove::<TableStore>();
		}
	}
}

/// A removed declaration takes its tables with it.
pub(crate) fn on_remove_json_tables(
	ev: On<Remove<JsonTableStore>>,
	mut commands: Commands,
) {
	commands.entity(ev.entity).try_remove::<TableStore>();
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn memory_store() {
		table_test::run(JsonTableStore::over(BlobStore::temp()).unwrap()).await;
	}

	/// A store under a path is created with the blob store holding it, and
	/// leaves what else it holds.
	#[beet_core::test]
	async fn under_a_path() {
		let blobs = BlobStore::temp();
		blobs.insert(&RelPath::new("other.txt"), "x").await.unwrap();
		let tables = JsonTableStore::default()
			.with_path(RelPath::new("tables"))
			.tables(&blobs)
			.unwrap();
		table_test::run_rows(&tables).await;
		let rows = tables.table::<table_test::NamedRow>();
		rows.put(table_test::NamedRow::new("a", 1)).await.unwrap();
		tables.store_remove().await.unwrap();
		blobs
			.list()
			.await
			.unwrap()
			.xpect_eq(vec![RelPath::new("other.txt")]);
	}

	/// The record-only cases a PDS answers hold of any backend holding
	/// record tables, so they are checked before a PDS runs them.
	#[beet_core::test]
	async fn record_cases() {
		table_test::run_records(TableStore::new(InMemoryTableStore::new()))
			.await
	}

	/// A PDS as a backend ([`TableProvider`]'s "A PDS as a backend") runs the
	/// record-only cases once the scene compiler's `PdsStore` implements the
	/// trait over `EmulatorPds`; until then this names them and runs nothing.
	#[beet_core::test]
	#[ignore = "runs once `PdsStore` implements `TableProvider` (the scene compiler plan)"]
	async fn pds_store() {}

	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	#[beet_core::test(timeout_ms = 60_000)]
	async fn fs_store() {
		let dir = TempDir::new_ws().unwrap();
		table_test::run(
			JsonTableStore::over(BlobStore::new(FsStore::new(
				dir.path().clone(),
			)))
			.unwrap(),
		)
		.await;
	}

	#[cfg(feature = "sqlite")]
	#[beet_core::test(timeout_ms = 60_000)]
	async fn sqlite_blob_store() {
		table_test::run(
			JsonTableStore::over(BlobStore::new(SqliteBlobStore::from(
				SqliteStore::memory("json-tables"),
			)))
			.unwrap(),
		)
		.await;
	}

	/// A partition is one pretty file with sorted keys, each row as it would
	/// be written by hand, and an emptied one is gone.
	#[beet_core::test]
	async fn layout() {
		let blobs = BlobStore::temp();
		let tables = JsonTableStore::over(blobs.clone()).unwrap();
		let rows = tables.table::<table_test::NamedRow>();
		rows.put(table_test::NamedRow::new("two", 2)).await.unwrap();
		rows.put(table_test::NamedRow::new("one", 1)).await.unwrap();
		tables
			.put(
				"bytes",
				StoredRow::new(
					"k",
					Row::from_value(value!({
						"body": (Value::Bytes(b"hi".to_vec())),
						"at": 0.1
					}))
					.unwrap(),
				)
				.unwrap(),
				Precondition::Any,
			)
			.await
			.unwrap();
		let partition = tables
			.partition("did:plc:abc")
			.unwrap()
			.partition("child")
			.unwrap()
			.table::<table_test::NamedRow>();
		partition
			.put(table_test::NamedRow::new("x", 1))
			.await
			.unwrap();
		let mut paths = blobs.list().await.unwrap();
		paths.sort();
		paths.xpect_eq(vec![
			RelPath::new("@root.json"),
			RelPath::new("did:plc:abc/child.json"),
		]);
		blobs
			.get(&RelPath::new("@root.json"))
			.await
			.unwrap()
			.xmap(|bytes| String::from_utf8(bytes.to_vec()).unwrap())
			.xpect_eq(
				r#"{
  "bytes": {
    "k": {
      "at": 0.1,
      "body": {
        "$bytes": "aGk"
      }
    }
  },
  "named_rows": {
    "one": {
      "key": "one",
      "value": 1
    },
    "two": {
      "key": "two",
      "value": 2
    }
  }
}
"#,
			);
		partition.remove("x").await.unwrap();
		blobs.list().await.unwrap().len().xpect_eq(1);
	}

	/// A file edited by hand is the table, and one that is not a partition is
	/// an error naming it, never read as empty or written over.
	#[beet_core::test]
	async fn reads_a_hand_written_file() {
		let blobs = BlobStore::temp();
		let tables = JsonTableStore::over(blobs.clone()).unwrap();
		let rows = tables.table::<table_test::NamedRow>();
		let file = RelPath::new("@root.json");
		blobs
			.insert(
				&file,
				r#"{"named_rows":{"b":{"value":2,"key":"b"},"a":{"key":"a","value":1}}}"#,
			)
			.await
			.unwrap();
		rows.list()
			.await
			.unwrap()
			.xpect_eq(vec!["a".into(), "b".into()]);
		// its cid is the content's, however it was written
		let (row, cid) = rows.get_with_cid("b").await.unwrap().unwrap();
		cid.clone()
			.xpect_eq(Row::from_serde(&row).unwrap().cid().unwrap());
		rows.put_if(row, Precondition::Matches(cid)).await.unwrap();
		for text in ["{\"a\":", "[]", r#"{"t":1}"#, r#"{"t":{"k":1}}"#] {
			blobs.insert(&file, text).await.unwrap();
			rows.get("a")
				.await
				.unwrap_err()
				.to_string()
				.xpect_contains("`@root.json`")
				.xpect_contains("is not a table partition");
			rows.put(table_test::NamedRow::new("c", 3))
				.await
				.unwrap_err();
			blobs
				.get(&file)
				.await
				.unwrap()
				.xpect_eq(bytes::Bytes::from(text));
		}
	}

	/// A row's cid read from json equals the one it was written with, for
	/// every value a row holds (`Row`'s own suite is the exhaustive one).
	#[beet_core::test]
	async fn cids_survive_the_file() {
		let tables = TableStore::new(InMemoryTableStore::new());
		let mut bits = 0x2545_f491_4f6c_dd1d_u64;
		for idx in 0..200 {
			bits ^= bits << 13;
			bits ^= bits >> 7;
			bits ^= bits << 17;
			let float = f64::from_bits(bits);
			let row = value!({
				"float": (match float.is_finite() { true => float, false => 1e16 }),
				"unit": ((bits >> 11) as f64 / (1u64 << 53) as f64),
				"uint": (bits),
				"int": (bits as i64),
				"bytes": (Value::Bytes(bits.to_be_bytes().to_vec()))
			});
			let row = StoredRow::new(
				format!("k{idx}"),
				Row::from_value(row).unwrap(),
			)
			.unwrap();
			tables
				.put("rows", row.clone(), Precondition::Any)
				.await
				.unwrap();
			tables
				.get("rows", row.key())
				.await
				.unwrap()
				.unwrap()
				.xpect_eq(row);
		}
	}

	/// Writers of unrelated rows race on one file and every row lands: a
	/// replaced file is retried, never a row's conflict.
	#[beet_core::test]
	async fn writers_of_unrelated_rows_never_conflict() {
		let blobs = BlobStore::temp();
		(0..16)
			.map(|idx| {
				// each its own handle, as another process's is
				let rows = JsonTableStore::over(blobs.clone())
					.unwrap()
					.table::<table_test::NamedRow>();
				async move {
					rows.push(table_test::NamedRow::new(format!("{idx}"), idx))
						.await
				}
			})
			.xmap(async_ext::try_join_all)
			.await
			.unwrap();
		JsonTableStore::over(blobs)
			.unwrap()
			.table::<table_test::NamedRow>()
			.list()
			.await
			.unwrap()
			.len()
			.xpect_eq(16);
	}

	/// Removing the tables beside anything else is refused, naming it.
	#[beet_core::test]
	async fn remove_refuses_what_is_not_a_table() {
		for (path, text) in [
			("media/cat.png", "png"),
			("notes.json", "[]"),
			("bad name.json", "{}"),
		] {
			let blobs = BlobStore::temp();
			let tables = JsonTableStore::over(blobs.clone()).unwrap();
			tables
				.table::<table_test::NamedRow>()
				.put(table_test::NamedRow::new("a", 1))
				.await
				.unwrap();
			blobs.insert(&RelPath::new(path), text).await.unwrap();
			tables
				.store_remove()
				.await
				.unwrap_err()
				.to_string()
				.xpect_contains(path)
				.xpect_contains("`path` of its own");
			blobs.list().await.unwrap().len().xpect_eq(2);
		}
	}

	/// A blob store with no conditional writes is refused, naming the fix.
	#[beet_core::test]
	fn refuses_a_store_without_conditional_writes() {
		JsonTableStore::over(BlobStore::new(StoreFork::new(
			BlobStore::temp(),
			BlobStore::temp(),
		)))
		.unwrap_err()
		.to_string()
		.xpect_contains("conditional writes")
		.xpect_contains("JsonTableStore")
		.xpect_contains("SqliteStore");
	}

	fn world() -> World { (AsyncPlugin, StorePlugin).into_world() }

	/// The backend an entity's own tables are kept in.
	fn tables(world: &World, entity: Entity) -> Option<String> {
		world.get::<TableStore>(entity).map(TableStore::describe)
	}

	/// A declaration is over the nearest blob store at or above it, and
	/// follows it as stores come, go and are parented.
	#[beet_core::test]
	fn follows_the_nearest_blob_store() {
		let mut world = world();
		let beside = world
			.spawn((InMemoryStore::named("beside"), JsonTableStore::default()))
			.flush();
		tables(&world, beside)
			.unwrap()
			.xpect_eq("json:memory:beside/@root.json");
		world.entity_mut(beside).remove::<JsonTableStore>();
		world.flush();
		tables(&world, beside).xpect_none();

		// parented after it spawned
		let child = world
			.spawn(JsonTableStore::default().with_path(RelPath::new("t")))
			.flush();
		tables(&world, child).xpect_none();
		let parent = world.spawn(InMemoryStore::named("parent")).flush();
		world.entity_mut(child).insert(ChildOf(parent));
		world.flush();
		tables(&world, child)
			.unwrap()
			.xpect_contains("json:memory:parent");
		// a nearer store
		world.entity_mut(child).insert(InMemoryStore::named("near"));
		world.flush();
		tables(&world, child)
			.unwrap()
			.xpect_contains("json:memory:near");
		// the store going away
		world.entity_mut(child).remove::<BlobStore>();
		world.flush();
		tables(&world, child).xpect_none();
	}

	/// An entity has one table store: a second backend beside the first is
	/// an error, whichever came first.
	#[cfg(feature = "sqlite")]
	#[beet_core::test]
	fn two_table_stores_are_refused() {
		static RAISED: std::sync::Mutex<Vec<String>> =
			std::sync::Mutex::new(Vec::new());
		let mut world = world();
		world.insert_resource(bevy::ecs::error::FallbackErrorHandler(
			|err, _| RAISED.lock().unwrap().push(err.to_string()),
		));
		let entity = world
			.spawn((InMemoryStore::new(), JsonTableStore::default()))
			.flush();
		let before = tables(&world, entity);
		world
			.entity_mut(entity)
			.insert(SqliteStore::memory("two-table-stores"));
		world.flush();
		tables(&world, entity).xpect_eq(before);
		RAISED.lock().unwrap()[0]
			.as_str()
			.xpect_contains("two table stores")
			.xpect_contains("sqlite:");
	}

	/// Neither kind of store lands the other, whichever backend it is.
	#[beet_core::test]
	fn neither_kind_implies_the_other() {
		let mut world = world();
		let memory = world.spawn(InMemoryStore::new()).flush();
		world.entity(memory).contains::<BlobStore>().xpect_true();
		world.entity(memory).contains::<TableStore>().xpect_false();
		let fs = world.spawn(FsStore::new(WsPath::new("target"))).flush();
		world.entity(fs).contains::<BlobStore>().xpect_true();
		world.entity(fs).contains::<TableStore>().xpect_false();
		let http = world
			.spawn(HttpStore::new(
				&Url::parse("http://localhost/repo").unwrap(),
			))
			.flush();
		world.entity(http).contains::<BlobStore>().xpect_true();
		world.entity(http).contains::<TableStore>().xpect_false();
		#[cfg(feature = "aws")]
		{
			let s3 = world.spawn(S3Store::new("b", "us-west-2")).flush();
			world.entity(s3).contains::<BlobStore>().xpect_true();
			world.entity(s3).contains::<TableStore>().xpect_false();
			let dynamo =
				world.spawn(DynamoStore::new("t", "us-west-2")).flush();
			world.entity(dynamo).contains::<TableStore>().xpect_true();
			world.entity(dynamo).contains::<BlobStore>().xpect_false();
		}
		// no plugin, whose watchers need a window
		#[cfg(target_arch = "wasm32")]
		{
			let mut world = World::new();
			for entity in [
				world.spawn(LocalStorageStore::new("one-kind")).flush(),
				world.spawn(IndexedDbStore::new("one-kind")).flush(),
			] {
				world.entity(entity).contains::<BlobStore>().xpect_true();
				world.entity(entity).contains::<TableStore>().xpect_false();
			}
		}
	}

	/// A table lookup passes a blob store on its way to the database above.
	#[cfg(feature = "sqlite")]
	#[beet_core::test]
	async fn a_lookup_passes_a_nested_blob_store() {
		let mut world = world();
		let database =
			world.spawn(SqliteStore::memory("nested-lookup")).flush();
		let directory = world
			.spawn((FsStore::new(WsPath::new("target")), ChildOf(database)))
			.flush();
		let leaf = world.spawn(ChildOf(directory)).flush();
		let found = world
			.run_async_then(move |world| async move {
				TableStore::nearest(&world.entity(leaf), "a test").await
			})
			.await
			.unwrap();
		found.describe().xpect_contains("nested-lookup");
		found.supports(Capability::Sql).xpect_true();
	}

	/// A lookup finding none names what asked and how to declare one.
	#[beet_core::test]
	async fn a_missing_table_store_names_its_fix() {
		let mut world = world();
		let entity = world.spawn(InMemoryStore::new()).flush();
		for err in [
			world
				.run_async_then(move |world| async move {
					TableStore::nearest(&world.entity(entity), "`Asker`").await
				})
				.await
				.unwrap_err(),
			world
				.run_async_then(move |world| async move {
					TableStore::declared(&world, entity, "`Asker`").await
				})
				.await
				.unwrap_err(),
		] {
			err.to_string()
				.xpect_contains("`Asker` needs a table store")
				.xpect_contains("<SqliteStore")
				.xpect_contains("{JsonTableStore}");
		}
	}
}
