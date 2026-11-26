//! [`InMemoryTableStore`]: tables in process memory.
use crate::prelude::*;
use beet_core::prelude::*;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;

/// Tables in process memory, gone with the last handle: for a test, or a
/// runtime whose rows need not outlive it.
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// # async fn run() -> Result<()> {
/// let tables = TableStore::new(InMemoryTableStore::new());
/// tables.table::<TableItem<u32>>().push(TableItem::new(7)).await?;
/// # Ok(())
/// # }
/// ```
///
/// - **Declared**, `<InMemoryTableStore/>` lands a [`TableStore`] beside it,
///   since a blob store never implies one.
/// - **Shared by its clones**: every handle cloned from one store reads and
///   writes the same rows. One built by reflection, as a scene loads, starts
///   empty.
/// - **Held as a [`JsonTableStore`] holds them**, a partition's tables whole,
///   so a batch is atomic and an index is a sort, with nothing serialized.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_insert = TableStore::on_insert::<Self>)]
pub struct InMemoryTableStore {
	/// Every partition's rows, shared by each handle on this store.
	#[reflect(ignore)]
	partitions: Arc<Mutex<HashMap<Option<PartitionId>, PartitionRows>>>,
	/// The partition this handle reads and writes, `None` for the store's own.
	#[reflect(ignore)]
	partition: Option<PartitionId>,
}

impl InMemoryTableStore {
	/// A new, empty store.
	pub fn new() -> Self { Self::default() }

	fn partitions(
		&self,
	) -> MutexGuard<'_, HashMap<Option<PartitionId>, PartitionRows>> {
		self.partitions
			.lock()
			.unwrap_or_else(|err| err.into_inner())
	}

	/// Read this partition's rows with `func`.
	fn read<T>(
		&self,
		func: impl FnOnce(&PartitionRows) -> Result<T>,
	) -> Result<T> {
		let partitions = self.partitions();
		match partitions.get(&self.partition) {
			Some(rows) => func(rows),
			None => func(&default()),
		}
	}
}

impl StoreBackend for InMemoryTableStore {
	fn id(&self) -> &'static str { "memory" }

	fn describe(&self) -> String {
		match &self.partition {
			Some(partition) => {
				format!("memory tables, partition `{partition}`")
			}
			None => "memory tables".into(),
		}
	}

	fn region(&self) -> Option<String> { None }

	fn store_exists(&self) -> SendBoxedFuture<Result<bool>> {
		Box::pin(async { Ok(true) })
	}

	fn store_create(&self) -> SendBoxedFuture<Result> {
		Box::pin(async { Ok(()) })
	}

	fn store_remove(&self) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move {
			this.partitions().clear();
			Ok(())
		})
	}
}

impl TableProvider for InMemoryTableStore {
	fn box_clone_table(&self) -> Box<dyn TableProvider> {
		Box::new(self.clone())
	}

	fn partition(&self, id: &PartitionId) -> Box<dyn TableProvider> {
		Box::new(Self {
			partitions: self.partitions.clone(),
			partition: Some(match &self.partition {
				Some(parent) => parent.join(id),
				None => id.clone(),
			}),
		})
	}

	fn supports(&self, capability: Capability) -> bool {
		matches!(capability, Capability::AtomicApply | Capability::Indexes)
	}

	fn get_row(
		&self,
		table: &TableSpec,
		key: &TableKey,
	) -> SendBoxedFuture<Result<Option<StoredRow>>> {
		let this = self.clone();
		let (table, key) = (table.clone(), key.clone());
		Box::pin(async move { this.read(|rows| rows.get(&table, &key)) })
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
		Box::pin(async move { this.read(|rows| rows.scan(&table, &scan)) })
	}

	fn apply(&self, writes: Vec<TableWrite>) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move {
			let mut partitions = this.partitions();
			let rows = partitions.entry(this.partition.clone()).or_default();
			match writes.as_slice() {
				// a failed write changes nothing, so one needs no copy
				[write] => {
					rows.apply(write)?;
				}
				writes => {
					let mut next = rows.clone();
					for write in writes {
						next.apply(write)?;
					}
					*rows = next;
				}
			}
			if rows.is_empty() {
				partitions.remove(&this.partition);
			}
			Ok(())
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
		let (table, index) = (table.clone(), index.clone());
		Box::pin(async move {
			this.read(|rows| rows.scan_index(&table, &index, &scan))
		})
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn passes_the_suite() {
		table_test::run(TableStore::new(InMemoryTableStore::new())).await;
	}

	#[beet_core::test]
	async fn passes_the_record_suite() {
		table_test::run_records(TableStore::new(InMemoryTableStore::new()))
			.await;
	}

	/// Declared, it lands a table store, and clones share their rows.
	#[beet_core::test]
	async fn lands_a_table_store() {
		let mut world = World::new();
		let store = InMemoryTableStore::new();
		let entity = world.spawn(store.clone()).id();
		world.flush();
		let landed = world.get::<TableStore>(entity).unwrap().clone();
		let rows = landed.table::<table_test::NamedRow>();
		rows.put(table_test::NamedRow::new("a", 1)).await.unwrap();
		TableStore::new(store)
			.table::<table_test::NamedRow>()
			.get("a")
			.await
			.unwrap()
			.xpect_some();
	}
}
