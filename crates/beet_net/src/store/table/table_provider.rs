use crate::prelude::*;
use beet_core::prelude::*;
use core::ops::Bound;
use serde::Deserialize;
use serde::Serialize;

/// Storage provider for one partition's tables: rows read and written as
/// [`StoredRow`]s, each write guarded by a [`Precondition`].
///
/// A provider is one database with one writer, atomic within itself:
/// [`partition`](Self::partition) scopes another one out of the same
/// declaration, the way [`with_subdir`](BlobStoreProvider::with_subdir) scopes
/// a blob store, and [`apply`](Self::apply) is atomic across this partition's
/// tables and never beyond them.
///
/// Optional capabilities are asked of a provider with
/// [`supports`](Self::supports), and one it lacks is refused with
/// [`TableError::Unsupported`]. Which backend supports what is the table
/// section of `crates/beet_net/README.md`.
///
/// Keys order by their raw bytes on every backend, so a scan answers the same
/// rows in the same order wherever the table lives.
///
/// # A PDS as a backend
///
/// An atproto PDS maps onto this surface with record tables only: a
/// partition is a repo, a table a collection (any other name refused), a key
/// an rkey. [`get_row`](Self::get_row) is `getRecord`;
/// [`put_row`](Self::put_row) is `putRecord` with `swapRecord` for
/// [`Precondition::Matches`] and `createRecord` for
/// [`Precondition::Absent`]; [`remove_row`](Self::remove_row) is
/// `deleteRecord` with `swapRecord`; [`scan_rows`](Self::scan_rows) is
/// `listRecords` with its cursor and `reverse`, end bounds applied client
/// side. [`apply`](Self::apply) is `applyWrites` guarded by `swapCommit`
/// after reading the records its preconditions name, stricter than per-row
/// and still correct. A PDS has no secondary indexes and refuses them.
pub trait TableProvider: BlobStoreProvider {
	/// Returns a boxed clone of this provider for type erasure.
	fn box_clone_table(&self) -> Box<dyn TableProvider>;

	/// The partition `id` of this declaration: its own database, isolated
	/// from this one and every other partition. Nests, so a partition of a
	/// partition is `parent/child`.
	fn partition(&self, id: &PartitionId) -> Box<dyn TableProvider>;

	/// Whether this backend supports `capability`.
	fn supports(&self, capability: Capability) -> bool;

	/// The row at `key` in `table`, `None` where there is none.
	fn get_row(
		&self,
		table: &TableSpec,
		key: &TableKey,
	) -> SendBoxedFuture<Result<Option<StoredRow>>>;

	/// Write `row` at its key in `table` when `precondition` holds of the row
	/// there now, else a [`TableError::Conflict`].
	fn put_row(
		&self,
		table: &TableSpec,
		row: StoredRow,
		precondition: Precondition,
	) -> SendBoxedFuture<Result>;

	/// Remove the row at `key` in `table` when `precondition` holds of it,
	/// else a [`TableError::Conflict`]. Removing a missing row under
	/// [`Precondition::Any`] succeeds.
	fn remove_row(
		&self,
		table: &TableSpec,
		key: &TableKey,
		precondition: Precondition,
	) -> SendBoxedFuture<Result>;

	/// One page of `table`'s rows in key order within `scan`'s bounds, and
	/// the cursor of the next page if there is one.
	fn scan_rows(
		&self,
		table: &TableSpec,
		scan: Scan,
	) -> SendBoxedFuture<Result<ScanPage>>;

	/// Apply every write or none, each under its own precondition, across
	/// this partition's tables. Refused by a backend that cannot make a batch
	/// atomic ([`Capability::AtomicApply`]).
	fn apply(&self, writes: Vec<TableWrite>) -> SendBoxedFuture<Result> {
		let _ = writes;
		let refusal = TableError::unsupported(self, Capability::AtomicApply);
		Box::pin(async move { Err(refusal.into()) })
	}

	/// Retire the rows at `keys` in `table` once a copy is safe elsewhere (an
	/// archive): gone from every read from now on. The default removes them;
	/// a backend that expires rows itself (Dynamo's `ttl`) marks them instead,
	/// so the removal costs nothing.
	fn expire_rows(
		&self,
		table: &TableSpec,
		keys: Vec<TableKey>,
	) -> SendBoxedFuture<Result> {
		let this = self.box_clone_table();
		let table = table.clone();
		Box::pin(async move {
			keys.into_iter()
				.map(|key| this.remove_row(&table, &key, Precondition::Any))
				.xmap(|removes| {
					async_ext::try_join_all_bounded(
						BlobStore::GET_ALL_CONCURRENCY,
						removes,
					)
				})
				.await?;
			Ok(())
		})
	}

	/// One page of `table`'s rows ordered by the declared index `index`,
	/// within `scan`'s bounds over its fields. Refused by a backend without
	/// secondary indexes ([`Capability::Indexes`]).
	fn scan_index(
		&self,
		table: &TableSpec,
		index: &TableIndex,
		scan: IndexScan,
	) -> SendBoxedFuture<Result<ScanPage>> {
		let _ = (table, index, scan);
		let refusal = TableError::unsupported(self, Capability::Indexes);
		Box::pin(async move { Err(refusal.into()) })
	}
}

/// What a write requires of the row already at its key, the atproto
/// `swapRecord` semantics: checked and applied as one step, so two writers
/// never silently erase each other.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Precondition {
	/// Whatever is there, including nothing.
	#[default]
	Any,
	/// Nothing is there.
	Absent,
	/// The row there has exactly this cid.
	Matches(Cid),
}

impl Precondition {
	/// Whether this holds of the row whose cid is `found`, `None` for no row.
	pub fn holds(&self, found: Option<&Cid>) -> bool {
		match (self, found) {
			(Self::Any, _) => true,
			(Self::Absent, found) => found.is_none(),
			(Self::Matches(expected), Some(found)) => expected == found,
			(Self::Matches(_), None) => false,
		}
	}

	/// [`holds`](Self::holds), else the conflict naming what was found.
	pub fn check(
		&self,
		table: &str,
		key: &TableKey,
		found: Option<&Cid>,
	) -> Result<(), TableError> {
		match self.holds(found) {
			true => Ok(()),
			false => Err(TableError::Conflict {
				table: table.into(),
				key: key.clone(),
				expected: self.clone(),
				found: found.cloned(),
			}),
		}
	}
}

impl core::fmt::Display for Precondition {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::Any => write!(f, "any row"),
			Self::Absent => write!(f, "no row"),
			Self::Matches(cid) => write!(f, "row {cid}"),
		}
	}
}

/// An optional capability a table backend may lack, asked of it with
/// [`TableProvider::supports`].
#[derive(
	Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect,
)]
pub enum Capability {
	/// [`TableProvider::apply`], every write of a batch or none.
	AtomicApply,
	/// Declared secondary indexes and [`TableProvider::scan_index`].
	Indexes,
	/// Arbitrary SQL beside the typed surface.
	Sql,
}

impl Capability {
	/// Every capability, for a backend listing what it supports.
	pub const ALL: [Self; 3] = [Self::AtomicApply, Self::Indexes, Self::Sql];
}

impl core::fmt::Display for Capability {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::AtomicApply => write!(f, "atomic batches"),
			Self::Indexes => write!(f, "secondary indexes"),
			Self::Sql => write!(f, "sql"),
		}
	}
}

/// The table errors a caller matches on: a failed precondition to retry, and
/// a capability or limit the backend refuses. Every other failure is a plain
/// error. Recover one from a [`BevyError`] with [`TableError::of`].
#[derive(Debug, Clone, PartialEq, thiserror::Error, Serialize, Deserialize)]
pub enum TableError {
	/// A write's [`Precondition`] did not hold.
	#[error("conflict at `{table}/{key}`: expected {expected}, found {}", found.as_ref().map_or("no row".to_string(), |cid| format!("row {cid}")))]
	Conflict {
		/// The table written.
		table: SmolStr,
		/// The key written.
		key: TableKey,
		/// What the write required.
		expected: Precondition,
		/// The cid there instead, `None` for no row.
		found: Option<Cid>,
	},
	/// The backend lacks a [`Capability`].
	#[error("`{backend}` does not support {capability}")]
	Unsupported {
		/// The backend, its [`describe`](BlobStoreProvider::describe).
		backend: SmolStr,
		/// What it lacks.
		capability: Capability,
	},
	/// A request past one of the backend's fixed limits.
	#[error("`{backend}` refuses {found} {what}, past its limit of {limit}")]
	Limit {
		/// The backend, its [`describe`](BlobStoreProvider::describe).
		backend: SmolStr,
		/// What is limited, ie `writes in one batch`.
		what: SmolStr,
		/// The backend's limit.
		limit: usize,
		/// What was asked for.
		found: usize,
	},
}

impl TableError {
	/// The refusal of `capability` by `provider`.
	pub fn unsupported(
		provider: &(impl ?Sized + BlobStoreProvider),
		capability: Capability,
	) -> Self {
		Self::Unsupported {
			backend: provider.describe().into(),
			capability,
		}
	}

	/// The table error `err` carries, if it is one.
	pub fn of(err: &BevyError) -> Option<&Self> { err.downcast_ref::<Self>() }

	/// Whether `err` is a failed precondition, the error a read-modify-write
	/// retries on.
	pub fn is_conflict(err: &BevyError) -> bool {
		matches!(Self::of(err), Some(Self::Conflict { .. }))
	}
}

/// A table as a provider sees it: its name, and the secondary indexes its
/// row type declares, which a backend with indexes maintains on every write.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TableSpec {
	/// [`TableStoreRow::table_name`].
	pub name: SmolStr,
	/// [`TableStoreRow::indexes`].
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub indexes: Vec<TableIndex>,
}

impl TableSpec {
	/// The table `name` with no indexes.
	pub fn new(name: impl Into<SmolStr>) -> Self {
		Self {
			name: name.into(),
			indexes: Vec::new(),
		}
	}

	/// The table a row type declares.
	pub fn of<T: TableStoreRow>() -> Self {
		Self {
			name: T::table_name(),
			indexes: T::indexes(),
		}
	}

	/// With these indexes.
	pub fn with_indexes(
		mut self,
		indexes: impl IntoIterator<Item = TableIndex>,
	) -> Self {
		self.indexes = indexes.into_iter().collect();
		self
	}

	/// The declared index `name`.
	pub fn index(&self, name: &str) -> Result<&TableIndex> {
		self.indexes
			.iter()
			.find(|index| index.name == name)
			.ok_or_else(|| {
				bevyhow!("table `{}` declares no index `{name}`", self.name)
			})
	}

	/// The NSID this table is named by, when it is a record table.
	pub fn collection(&self) -> Option<Nsid> { Nsid::parse(&self.name).ok() }
}

impl From<&str> for TableSpec {
	fn from(name: &str) -> Self { Self::new(name) }
}
impl From<SmolStr> for TableSpec {
	fn from(name: SmolStr) -> Self { Self::new(name) }
}
impl From<&TableSpec> for TableSpec {
	fn from(spec: &TableSpec) -> Self { spec.clone() }
}

/// The name of one partition: 1 to 512 characters of `A-Za-z0-9._:~-` per
/// segment (an rkey's alphabet, so a DID is one), segments joined by `/` for
/// a partition of a partition. Every backend addresses it as given: a path
/// segment, a database file name, a Durable Object's name.
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Reflect,
	StringPrimitive,
	Serialize,
	Deserialize,
)]
#[serde(try_from = "SmolStr", into = "SmolStr")]
pub struct PartitionId(SmolStr);

impl PartitionId {
	/// Parse a partition id, refusing a segment no backend could address.
	pub fn parse(text: &str) -> Result<Self> {
		for segment in text.split('/') {
			Rkey::parse(segment).map_err(|err| {
				bevyhow!("invalid partition id `{text}`: {err}")
			})?;
		}
		Self(text.into()).xok()
	}

	/// This partition's child `id`.
	pub fn join(&self, id: &PartitionId) -> Self {
		Self(format!("{}/{}", self.0, id.0).into())
	}

	/// Each segment, outermost first.
	pub fn segments(&self) -> impl Iterator<Item = &str> { self.0.split('/') }
}

/// The bounds, direction and page of a [`TableProvider::scan_rows`]. Bounds
/// compare keys as raw bytes; an unbounded scan reads the whole table a page
/// at a time.
///
/// ```
/// # use beet_net::prelude::*;
/// let scan = Scan::default().from("a").before("m").limit(10).reverse();
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scan {
	/// The lowest key, whichever the direction.
	pub start: Bound<TableKey>,
	/// The highest key, whichever the direction.
	pub end: Bound<TableKey>,
	/// The most rows a page answers; a backend may answer fewer, with a
	/// cursor.
	pub limit: Option<usize>,
	/// Highest key first.
	pub reverse: bool,
	/// Where the previous page stopped, see [`ScanPage::cursor`].
	pub cursor: Option<ScanCursor>,
}

impl Default for Scan {
	fn default() -> Self {
		Self {
			start: Bound::Unbounded,
			end: Bound::Unbounded,
			limit: None,
			reverse: false,
			cursor: None,
		}
	}
}

impl Scan {
	/// Keys from `key`, inclusive.
	pub fn from(mut self, key: impl Into<TableKey>) -> Self {
		self.start = Bound::Included(key.into());
		self
	}
	/// Keys after `key`.
	pub fn after(mut self, key: impl Into<TableKey>) -> Self {
		self.start = Bound::Excluded(key.into());
		self
	}
	/// Keys up to `key`, inclusive.
	pub fn through(mut self, key: impl Into<TableKey>) -> Self {
		self.end = Bound::Included(key.into());
		self
	}
	/// Keys before `key`.
	pub fn before(mut self, key: impl Into<TableKey>) -> Self {
		self.end = Bound::Excluded(key.into());
		self
	}
	/// Keys nested under `parent` at any depth, ie `threads/a/1` and
	/// `threads/a/b/2` under `threads/a`, never `threads/ab/3`. Every key
	/// nests under an empty parent.
	pub fn under(self, parent: impl AsRef<str>) -> Self {
		match parent.as_ref().trim_end_matches('/') {
			"" => self,
			// `0` is the byte after `/`
			parent => {
				self.from(format!("{parent}/")).before(format!("{parent}0"))
			}
		}
	}
	/// At most `limit` rows a page.
	pub fn limit(mut self, limit: usize) -> Self {
		self.limit = Some(limit);
		self
	}
	/// Highest key first.
	pub fn reverse(mut self) -> Self {
		self.reverse = true;
		self
	}
	/// Continue after a previous page.
	pub fn with_cursor(mut self, cursor: Option<ScanCursor>) -> Self {
		self.cursor = cursor;
		self
	}

	/// Whether `key` lies within the bounds.
	pub fn contains(&self, key: &TableKey) -> bool {
		let above = match &self.start {
			Bound::Included(start) => key >= start,
			Bound::Excluded(start) => key > start,
			Bound::Unbounded => true,
		};
		let below = match &self.end {
			Bound::Included(end) => key <= end,
			Bound::Excluded(end) => key < end,
			Bound::Unbounded => true,
		};
		above && below
	}

	/// The bounds narrowed past a cursor that is the last key a page
	/// answered, the cursor a backend without its own answers.
	pub fn after_key_cursor(&self) -> Self {
		let mut scan = self.clone();
		if let Some(cursor) = scan.cursor.take() {
			let key = TableKey::from(cursor.as_str());
			match scan.reverse {
				false => scan.start = Bound::Excluded(key),
				true => scan.end = Bound::Excluded(key),
			}
		}
		scan
	}
}

/// Where a scan stopped, opaque to the caller: handed back with the same
/// bounds, it answers the next page.
#[derive(
	Debug, Clone, PartialEq, Eq, Hash, Deref, Serialize, Deserialize, Reflect,
)]
#[serde(transparent)]
pub struct ScanCursor(SmolStr);

impl ScanCursor {
	/// A cursor carrying `text`, the backend's own encoding.
	pub fn new(text: impl Into<SmolStr>) -> Self { Self(text.into()) }
	/// The backend's encoding.
	pub fn as_str(&self) -> &str { &self.0 }
}

/// One page of a scan: its rows and the cursor of the next, `None` on the
/// last page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TablePage<T> {
	/// The rows, in scan order.
	pub rows: Vec<T>,
	/// The next page's cursor, `None` when this is the last.
	pub cursor: Option<ScanCursor>,
}

impl<T> Default for TablePage<T> {
	fn default() -> Self {
		Self {
			rows: Vec::new(),
			cursor: None,
		}
	}
}

impl<T> TablePage<T> {
	/// The same page with each row mapped.
	pub fn try_map<U>(
		self,
		func: impl FnMut(T) -> Result<U>,
	) -> Result<TablePage<U>> {
		TablePage {
			rows: self.rows.into_iter().map(func).collect::<Result<_>>()?,
			cursor: self.cursor,
		}
		.xok()
	}
}

/// A page of stored rows, what a provider answers.
pub type ScanPage = TablePage<StoredRow>;

impl ScanPage {
	/// The page of the rows in `rows` (already filtered to the scan's bounds
	/// and sorted ascending) that `scan` answers, its cursor the last key
	/// answered: the scan of a backend that lists rather than queries.
	pub fn from_sorted(mut rows: Vec<StoredRow>, scan: &Scan) -> Self {
		if scan.reverse {
			rows.reverse();
		}
		let more = scan.limit.is_some_and(|limit| rows.len() > limit);
		if let Some(limit) = scan.limit {
			rows.truncate(limit);
		}
		let cursor = more
			.then(|| rows.last().map(|row| ScanCursor::new(row.key().as_str())))
			.flatten();
		Self { rows, cursor }
	}
}

/// One write of a [`TableProvider::apply`] batch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TableWrite {
	/// Write a row at its key.
	Put {
		/// The table written.
		table: TableSpec,
		/// The row.
		row: StoredRow,
		/// What the row there now must be.
		precondition: Precondition,
	},
	/// Remove the row at a key.
	Remove {
		/// The table written.
		table: TableSpec,
		/// The key.
		key: TableKey,
		/// What the row there now must be.
		precondition: Precondition,
	},
}

impl TableWrite {
	/// The table this write names.
	pub fn table(&self) -> &TableSpec {
		match self {
			Self::Put { table, .. } | Self::Remove { table, .. } => table,
		}
	}
	/// The key this write names.
	pub fn key(&self) -> &TableKey {
		match self {
			Self::Put { row, .. } => row.key(),
			Self::Remove { key, .. } => key,
		}
	}
	/// What this write requires.
	pub fn precondition(&self) -> &Precondition {
		match self {
			Self::Put { precondition, .. }
			| Self::Remove { precondition, .. } => precondition,
		}
	}
}
