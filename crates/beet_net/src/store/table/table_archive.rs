use crate::prelude::*;
use beet_core::prelude::*;

/// Moves a table's cold rows out of it: every row in a key range copied into
/// a SQLite database (the [`SqliteStore`] schema, each row's cid carried and
/// never recomputed), the copy verified row for row, written to a bucket as
/// one file and verified byte for byte, and only then the rows retired from
/// the table ([`TableStore::expire`], on Dynamo a `ttl` stamp so the deletion
/// is free).
///
/// The database is built in memory and serialized ([`SqliteStore::to_bytes`]),
/// so no file is written locally and the archive runs wherever SQLite does: a
/// native process, a browser, and a Cloudflare Worker, whose `wasm32` build
/// links `sqlite-wasm-rs` and holds the database in its memory VFS.
///
/// Rerunning a range is safe: an archive already at the path is merged with
/// the rows still in the table, and a range with no rows left writes nothing.
/// [`open`](Self::open) reads an archive back as a read-only [`SqliteStore`],
/// queryable with [`SqlSelect`] and the same [`Table`] reads as the live
/// table.
///
/// ```no_run
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// # async fn run(store: TableStore, bucket: BlobStore) -> Result<()> {
/// let events = store.table::<TableItem<u32>>();
/// let path = RelPath::new("archive/2026-09.db");
/// TableArchive::new(&events, Scan::default().before("3m"))
/// 	.run(&bucket, &path)
/// 	.await?;
/// let cold = TableArchive::open(&bucket, &path).await?;
/// Table::<TableItem<u32>>::new(cold).get_all().await?;
/// # Ok(())
/// # }
/// ```
pub struct TableArchive {
	store: TableStore,
	table: TableSpec,
	scan: Scan,
}

/// What a [`TableArchive`] run moved.
#[derive(Debug, Clone, PartialEq)]
pub struct TableArchiveReport {
	/// Where the archive was written.
	pub path: RelPath,
	/// The rows moved out of the table this run.
	pub rows: usize,
	/// The archive file's size, `0` when there was nothing to move.
	pub bytes: usize,
}

impl TableArchive {
	/// Archive the rows of `table` within `scan`'s bounds (its limit,
	/// direction and cursor are ignored).
	pub fn new<T: TableStoreRow>(table: &Table<T>, scan: Scan) -> Self {
		Self {
			store: table.store().clone(),
			table: table.spec().clone(),
			scan: Scan {
				limit: None,
				reverse: false,
				cursor: None,
				..scan
			},
		}
	}

	/// Copy, verify, write and retire, see [`TableArchive`].
	pub async fn run(
		&self,
		bucket: &BlobStore,
		path: &RelPath,
	) -> Result<TableArchiveReport> {
		let mut rows = self.scan_all(&self.store).await?;
		let moved =
			rows.iter().map(|row| row.key().clone()).collect::<Vec<_>>();
		if moved.is_empty() {
			return TableArchiveReport {
				path: path.clone(),
				rows: 0,
				bytes: 0,
			}
			.xok();
		}
		// an archive already here keeps its rows, the table's winning a key
		// both hold
		if bucket.exists(path).await? {
			let existing = TableStore::new(Self::open(bucket, path).await?);
			let mut merged = self.scan_all(&existing).await?;
			merged.retain(|row| !moved.contains(row.key()));
			merged.extend(rows);
			merged.sort_by(|left, right| left.key().cmp(right.key()));
			rows = merged;
		}
		let bytes = self.build(&rows).await?;
		bucket.insert(path, bytes.clone()).await?;
		if bucket.get(path).await? != bytes {
			bevybail!(
				"table archive `{path}` was written to {} but failed read-back verification",
				bucket.describe()
			);
		}
		self.store.expire(&self.table, moved.clone()).await?;
		TableArchiveReport {
			path: path.clone(),
			rows: moved.len(),
			bytes: bytes.len(),
		}
		.xok()
	}

	/// The archive at `path` in `bucket` as a read-only [`SqliteStore`], a
	/// copy in memory of its own, so two opens never share one;
	/// [`store_remove`](StoreBackend::store_remove) frees it.
	pub async fn open(
		bucket: &BlobStore,
		path: &RelPath,
	) -> Result<SqliteStore> {
		let bytes = bucket.get(path).await?;
		SqliteStore::from_bytes(
			format!(
				"table-archive-open-{}/{path}",
				FastRandom::default().random::<u64>()
			),
			bytes.to_vec(),
		)
		.await
	}

	/// Every row of the table within the bounds in `store`.
	async fn scan_all(&self, store: &TableStore) -> Result<Vec<StoredRow>> {
		let mut scan = self.scan.clone();
		let mut rows = Vec::new();
		loop {
			let page = store.scan(&self.table, scan.clone()).await?;
			rows.extend(page.rows);
			match page.cursor {
				Some(cursor) => scan.cursor = Some(cursor),
				None => break,
			}
		}
		rows.xok()
	}

	/// The database holding `rows`, built in memory, verified row for row
	/// against them, and serialized.
	async fn build(&self, rows: &[StoredRow]) -> Result<Vec<u8>> {
		let database = SqliteStore::memory(format!(
			"table-archive-{}",
			FastRandom::default().random::<u64>()
		));
		let tables = TableStore::new(database.clone());
		let result = async {
			tables
				.apply(
					rows.iter()
						.map(|row| TableWrite::Put {
							table: self.table.clone(),
							row: row.clone(),
							precondition: Precondition::Any,
						})
						.collect(),
				)
				.await?;
			let copied = Self {
				store: tables.clone(),
				table: self.table.clone(),
				scan: Scan::default(),
			}
			.scan_all(&tables)
			.await?;
			// the content hashed again, not only the cid carried with it
			let contents = |rows: &[StoredRow]| {
				rows.iter()
					.map(|row| {
						(row.key().clone(), row.cid().clone(), row.row().cid()?)
							.xok()
					})
					.collect::<Result<Vec<_>>>()
			};
			if contents(&copied)? != contents(rows)? {
				bevybail!(
					"table archive of `{}` failed verification: the copy holds {} rows of {}",
					self.table.name,
					copied.len(),
					rows.len()
				);
			}
			database.to_bytes().await
		}
		.await;
		database.store_remove().await?;
		result
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	fn rows(store: &TableStore) -> Table<table_test::NamedRow> {
		store.table::<table_test::NamedRow>()
	}

	/// A range moves whole into a file, readable as the same table with its
	/// cids, and leaves the table; the rest stays.
	#[beet_core::test]
	async fn moves_a_range() {
		let store = TableStore::new(InMemoryTableStore::new());
		let bucket = BlobStore::temp();
		let table = rows(&store);
		for (key, value) in [("a", 1), ("b", 2), ("c", 3)] {
			table
				.put(table_test::NamedRow::new(key, value))
				.await
				.unwrap();
		}
		let cid = table.get_with_cid("a").await.unwrap().unwrap().1;
		let path = RelPath::new("archive/named.db");
		let report = TableArchive::new(&table, Scan::default().before("c"))
			.run(&bucket, &path)
			.await
			.unwrap();
		report.rows.xpect_eq(2);
		(report.bytes > 0).xpect_true();
		table
			.list()
			.await
			.unwrap()
			.xpect_eq(vec![TableKey::from("c")]);

		let cold = TableArchive::open(&bucket, &path).await.unwrap();
		let archived = Table::<table_test::NamedRow>::new(cold.clone());
		archived
			.list()
			.await
			.unwrap()
			.xpect_eq(vec!["a".into(), "b".into()]);
		archived
			.get_with_cid("a")
			.await
			.unwrap()
			.unwrap()
			.1
			.xpect_eq(cid);
		// queryable with SQL, and read only
		cold.query("SELECT key FROM named_rows ORDER BY key")
			.await
			.unwrap()
			.len()
			.xpect_eq(2);
		archived
			.put(table_test::NamedRow::new("z", 0))
			.await
			.unwrap_err();
	}

	/// A rerun merges into the archive already there, and an empty range
	/// writes nothing over it.
	#[beet_core::test]
	async fn reruns_merge() {
		let store = TableStore::new(InMemoryTableStore::new());
		let bucket = BlobStore::temp();
		let table = rows(&store);
		let path = RelPath::new("archive/named.db");
		let archive = TableArchive::new(&table, Scan::default());
		table.put(table_test::NamedRow::new("a", 1)).await.unwrap();
		archive.run(&bucket, &path).await.unwrap();
		archive.run(&bucket, &path).await.unwrap().rows.xpect_eq(0);
		table.put(table_test::NamedRow::new("b", 2)).await.unwrap();
		archive.run(&bucket, &path).await.unwrap().rows.xpect_eq(1);
		Table::<table_test::NamedRow>::new(
			TableArchive::open(&bucket, &path).await.unwrap(),
		)
		.list()
		.await
		.unwrap()
		.xpect_eq(vec!["a".into(), "b".into()]);
	}
}
