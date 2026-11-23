//! Any [`BlobStore`] as a table backend: a row is one object at
//! `<table>/<key>` holding its [`StoredRow::to_stored_json`] text, a
//! partition the subdirectory `@<id>`, so no partition ever shares a path
//! with a table (table names never begin with `@`).
//!
//! Every precondition is checked and written as one step through the blob
//! store's own conditional write ([`BlobStoreProvider::insert_if`]), its
//! version token being the store's (an S3 ETag, a digest under a local
//! store's lock), so nothing interleaves undetected. A scan lists the table's
//! prefix and sorts it by key bytes, so every page lists the table: cheap for
//! a small table, linear for a large one, and a reverse page lists the same
//! range. There are no atomic batches and no secondary indexes, both refused
//! with [`TableError::Unsupported`].
use crate::prelude::*;
use beet_core::prelude::*;

impl BlobStore {
	/// Where a row of `table` at `key` lives in this store, refusing a key
	/// whose segments a path cannot hold.
	fn row_path(table: &TableSpec, key: &TableKey) -> Result<RelPath> {
		if key
			.split('/')
			.any(|segment| matches!(segment, "" | "." | ".."))
		{
			bevybail!(
				"a blob backed table cannot hold key `{key}`: its `/` \
				 separated segments must be non-empty and not `.` or `..`"
			);
		}
		RelPath::new(table.name.as_str()).join(key.as_str()).xok()
	}

	/// The stored row at `path` with the blob's version, `None` where there
	/// is none.
	async fn versioned_row(
		&self,
		key: &TableKey,
		path: &RelPath,
	) -> Result<Option<(StoredRow, BlobVersion)>> {
		match self.get_versioned(path).await? {
			Some((bytes, version)) => Some((
				StoredRow::from_stored_json(
					key.clone(),
					core::str::from_utf8(&bytes)?,
				)?,
				version,
			)),
			None => None,
		}
		.xok()
	}

	/// The blob condition a row precondition becomes, read against the row
	/// there now: a cid compared here, then written on the object's own
	/// version so nothing can change it between the read and the write.
	async fn blob_condition(
		&self,
		table: &TableSpec,
		key: &TableKey,
		path: &RelPath,
		precondition: &Precondition,
	) -> Result<BlobCondition> {
		match precondition {
			Precondition::Any => BlobCondition::Any,
			Precondition::Absent => BlobCondition::Absent,
			Precondition::Matches(_) => {
				let current = self.versioned_row(key, path).await?;
				precondition.check(
					&table.name,
					key,
					current.as_ref().map(|(row, _)| row.cid()),
				)?;
				let (_, version) = current.expect("a cid matched");
				BlobCondition::Matches(version)
			}
		}
		.xok()
	}

	/// A blob conflict as the table conflict it is, naming the row found.
	async fn conflict(
		&self,
		err: BevyError,
		table: &TableSpec,
		key: &TableKey,
		path: &RelPath,
		precondition: &Precondition,
	) -> BevyError {
		if err.downcast_ref::<BlobConflict>().is_none() {
			return err;
		}
		let found = match self.versioned_row(key, path).await {
			Ok(found) => found.map(|(row, _)| row.cid().clone()),
			Err(err) => return err,
		};
		TableError::Conflict {
			table: table.name.clone(),
			key: key.clone(),
			expected: precondition.clone(),
			found,
		}
		.into()
	}
}

impl TableProvider for BlobStore {
	fn box_clone_table(&self) -> Box<dyn TableProvider> {
		Box::new(self.clone())
	}

	fn partition(&self, id: &PartitionId) -> Box<dyn TableProvider> {
		let path = id
			.segments()
			.map(|segment| format!("@{segment}"))
			.collect::<Vec<_>>()
			.join("/");
		Box::new(self.with_subdir(RelPath::new(path)))
	}

	fn supports(&self, _capability: Capability) -> bool { false }

	fn get_row(
		&self,
		table: &TableSpec,
		key: &TableKey,
	) -> SendBoxedFuture<Result<Option<StoredRow>>> {
		let this = self.clone();
		let table = table.clone();
		let key = key.clone();
		Box::pin(async move {
			let path = Self::row_path(&table, &key)?;
			this.versioned_row(&key, &path)
				.await?
				.map(|(row, _)| row)
				.xok()
		})
	}

	fn put_row(
		&self,
		table: &TableSpec,
		row: StoredRow,
		precondition: Precondition,
	) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let table = table.clone();
		Box::pin(async move {
			let key = row.key().clone();
			let path = Self::row_path(&table, &key)?;
			let condition = this
				.blob_condition(&table, &key, &path, &precondition)
				.await?;
			let body = row.to_stored_json()?;
			match this.insert_if(&path, body.into(), condition).await {
				Ok(_) => Ok(()),
				Err(err) => Err(this
					.conflict(err, &table, &key, &path, &precondition)
					.await),
			}
		})
	}

	fn remove_row(
		&self,
		table: &TableSpec,
		key: &TableKey,
		precondition: Precondition,
	) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let table = table.clone();
		let key = key.clone();
		Box::pin(async move {
			let path = Self::row_path(&table, &key)?;
			let condition = this
				.blob_condition(&table, &key, &path, &precondition)
				.await?;
			match this.remove_if(&path, condition).await {
				Ok(()) => Ok(()),
				Err(err) => Err(this
					.conflict(err, &table, &key, &path, &precondition)
					.await),
			}
		})
	}

	fn scan_rows(
		&self,
		table: &TableSpec,
		scan: Scan,
	) -> SendBoxedFuture<Result<ScanPage>> {
		let this = self.clone();
		let table = table.clone();
		Box::pin(async move {
			let bounded = scan.after_key_cursor();
			let mut keys = this
				.with_subdir(RelPath::new(table.name.as_str()))
				.list()
				.await?
				.into_iter()
				.map(|path| TableKey::from(path.as_str()))
				.filter(|key| bounded.contains(key))
				.collect::<Vec<_>>();
			keys.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
			if bounded.reverse {
				keys.reverse();
			}
			// one past the limit says whether there is another page
			if let Some(limit) = bounded.limit {
				keys.truncate(limit + 1);
			}
			let mut rows = keys
				.into_iter()
				.map(|key| {
					let this = this.clone();
					let table = table.clone();
					async move {
						let path = Self::row_path(&table, &key)?;
						this.versioned_row(&key, &path)
							.await
							.map(|row| row.map(|(row, _)| row))
					}
				})
				.xmap(|rows| {
					async_ext::try_join_all_bounded(
						BlobStore::GET_ALL_CONCURRENCY,
						rows,
					)
				})
				.await?
				.into_iter()
				// a row removed between the listing and its read
				.flatten()
				.collect::<Vec<_>>();
			if bounded.reverse {
				rows.reverse();
			}
			ScanPage::from_sorted(rows, &bounded).xok()
		})
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn memory_store() {
		table_test::run(TableStore::new(BlobStore::temp())).await;
	}

	/// The record-only cases a PDS answers hold of any backend holding
	/// record tables, so they are checked before a PDS runs them.
	#[beet_core::test]
	async fn record_cases() {
		table_test::run_records(TableStore::new(BlobStore::temp())).await;
	}

	/// A PDS as a backend ([`TableProvider`]'s "A PDS as a backend") runs the
	/// record-only cases once the scene compiler's `PdsStore` implements the
	/// trait over `EmulatorPds`; until then this names them and runs nothing.
	#[beet_core::test]
	#[ignore = "runs once `PdsStore` implements `TableProvider` (the scene compiler plan)"]
	async fn pds_store() {}

	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	#[beet_core::test]
	async fn fs_store() {
		let dir = TempDir::new_ws().unwrap();
		table_test::run(TableStore::new(BlobStore::new(FsStore::new(
			dir.path().clone(),
		))))
		.await;
	}

	/// A row is one object at `<table>/<key>` holding its stored text, a
	/// partition the `@<id>` subdirectory.
	#[beet_core::test]
	async fn layout() {
		let store = BlobStore::temp();
		let tables = TableStore::new(store.clone());
		tables
			.table::<table_test::NamedRow>()
			.put(table_test::NamedRow::new("one", 7))
			.await
			.unwrap();
		tables
			.partition("did:plc:abc")
			.unwrap()
			.table::<table_test::NamedRow>()
			.put(table_test::NamedRow::new("two", 8))
			.await
			.unwrap();
		let mut paths = store.list().await.unwrap();
		paths.sort();
		paths.xpect_eq(vec![
			RelPath::new("@did:plc:abc/named_rows/two"),
			RelPath::new("named_rows/one"),
		]);
		store
			.get(&RelPath::new("named_rows/one"))
			.await
			.unwrap()
			.xmap(|bytes| String::from_utf8(bytes.to_vec()).unwrap())
			.xpect_starts_with(r#"{"cid":"bafyrei"#)
			.xpect_contains(r#""row":{"key":"one","value":7}"#);
	}

	/// The materialized currency: any store component lands a [`TableStore`].
	#[beet_core::test]
	fn provider_component_materializes_table_store() {
		let mut world = World::new();
		let entity = world.spawn(InMemoryStore::new()).id();
		world.flush();
		world.entity(entity).contains::<TableStore>().xpect_true();
	}
}
