//! The table conformance suite every [`TableProvider`] runs: [`run`] for a
//! backend holding any table, [`run_records`] for one holding record tables
//! only (a PDS).
use crate::prelude::*;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;

/// A row keyed by a string it carries, in a table it names itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedRow {
	/// The row's key.
	pub key: String,
	/// A counter or tag.
	pub value: i64,
}

impl NamedRow {
	/// The row `key` holding `value`.
	pub fn new(key: impl Into<String>, value: i64) -> Self {
		Self {
			key: key.into(),
			value,
		}
	}
}

impl TableStoreRow for NamedRow {
	fn table_name() -> SmolStr { "named_rows".into() }
	fn key(&self) -> TableKey { self.key.as_str().into() }
}

/// A second table, to prove tables disjoint and batches spanning them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OtherRow {
	/// The row's key.
	pub key: String,
}

impl TableStoreRow for OtherRow {
	fn key(&self) -> TableKey { self.key.as_str().into() }
}

/// A row with a secondary index over two of its fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexedRow {
	/// The row's key.
	pub key: String,
	/// The first indexed field, any scalar.
	pub group: Value,
	/// The second indexed field.
	pub rank: i64,
}

impl TableStoreRow for IndexedRow {
	fn key(&self) -> TableKey { self.key.as_str().into() }
	fn indexes() -> Vec<TableIndex> {
		vec![
			TableIndex::new("by_group", ["group", "rank"]),
			TableIndex::new("by_rank", ["rank"]),
		]
	}
}

/// A record body, a float among its fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
	/// The note's text.
	pub text: String,
	/// A float, which crosses as the data model's float object.
	pub scale: f64,
}

impl AtprotoRecord for Note {
	const COLLECTION: Nsid = nsid!("com.example.note");
}

/// A record body naming another collection than its own.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mislabeled {
	#[serde(rename = "$type")]
	/// The collection it claims.
	pub r#type: SmolStr,
}

impl AtprotoRecord for Mislabeled {
	const COLLECTION: Nsid = nsid!("com.example.note");
}

/// Values in ascending order, every kind an index holds, ties across
/// kinds and the integers a float cannot represent included.
pub fn ordered_values() -> Vec<Value> {
	vec![
		Value::Null,
		Value::Float(-1e300),
		Value::Int(i64::MIN),
		Value::Int(-9_007_199_254_740_993),
		Value::Float(-9_007_199_254_740_992.0),
		Value::Int(-3),
		Value::Float(-0.5),
		Value::Int(0),
		Value::Float(0.25),
		Value::Int(1),
		Value::Float(1.5),
		Value::Int(9_007_199_254_740_992),
		Value::Int(9_007_199_254_740_993),
		Value::Int(i64::MAX),
		Value::Float(1e300),
		Value::Str("".into()),
		Value::Str("\0".into()),
		Value::Str("A".into()),
		Value::Str("Z".into()),
		Value::Str("a".into()),
		Value::Str("a\0b".into()),
		Value::Str("ab".into()),
		Value::Str("é".into()),
		Value::Str("日本".into()),
	]
}

/// Keys whose byte order differs from any case-folded, numeric or
/// code-point-insensitive order, in byte order.
pub fn ordered_keys() -> Vec<&'static str> {
	let mut keys = vec![
		"10", "9", "A", "B", "Z", "_", "a", "aa", "b", "z", "~", "é", "ê",
		"日本", "😀",
	];
	keys.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
	keys
}

/// Runs the whole suite against `store`, a backend holding any table,
/// leaving it removed.
pub async fn run(store: TableStore) {
	store.store_remove().await.ok();
	store.store_try_create().await.unwrap();
	run_rows(&store).await;
	store.store_remove().await.unwrap();
}

/// Runs every case but the store's lifecycle, for a store whose creation and
/// removal are its operator's (a Worker's bucket binding), leaving it as
/// empty as it found it.
pub async fn run_rows(store: &TableStore) {
	let store = store.clone();
	round_trip(&store).await;
	preconditions(&store).await;
	concurrent_writes(&store).await;
	scans(&store).await;
	partitions(&store).await;
	records(&store).await;
	atomic_apply(&store).await;
	indexes(&store).await;
	expiry(&store).await;
}

/// Runs the cases a record-only backend (a PDS) answers: record tables,
/// their preconditions, scans and partitions, everything keyed by rkey.
pub async fn run_records(store: TableStore) {
	records(&store).await;
	let notes = store.table::<Rkeyed<Note>>();
	let note = |key: &str, text: &str| {
		Rkeyed::new(Rkey::parse(key).unwrap(), Note {
			text: text.into(),
			scale: 1.0,
		})
	};
	let cid = notes.push(note("a", "one")).await.unwrap();
	notes.push(note("a", "two")).await.xpect_conflict();
	notes
		.put_if(note("a", "two"), Precondition::Matches(Cid::raw(b"x")))
		.await
		.xpect_conflict();
	notes
		.put_if(note("a", "two"), Precondition::Matches(cid))
		.await
		.unwrap();
	for key in ["b", "c", "d"] {
		notes.push(note(key, key)).await.unwrap();
	}
	let page = notes.scan(Scan::default().limit(2)).await.unwrap();
	page.rows.len().xpect_eq(2);
	notes
		.scan_all(Scan::default().reverse())
		.await
		.unwrap()
		.into_iter()
		.map(|note| note.rkey().to_string())
		.collect::<Vec<_>>()
		.xpect_eq(vec!["d", "c", "b", "a"]);
	let other = store.partition("other").unwrap().table::<Rkeyed<Note>>();
	other.get("a").await.unwrap().xpect_none();
	for key in ["a", "b", "c", "d"] {
		notes.remove(key).await.unwrap();
	}
}

/// What a conflict asserts.
trait XpectConflict {
	fn xpect_conflict(self);
}
impl<T: core::fmt::Debug> XpectConflict for Result<T> {
	#[track_caller]
	fn xpect_conflict(self) {
		let err = self.unwrap_err();
		if !TableError::is_conflict(&err) {
			panic!("expected a conflict, found: {err}");
		}
	}
}

/// What a refusal of `capability` asserts.
#[track_caller]
fn xpect_unsupported<T: core::fmt::Debug>(
	result: Result<T>,
	capability: Capability,
) {
	let err = result.unwrap_err();
	match TableError::of(&err) {
		Some(TableError::Unsupported {
			capability: refused,
			..
		}) if *refused == capability => {}
		_ => panic!("expected {capability} refused, found: {err}"),
	}
}

async fn round_trip(store: &TableStore) {
	let rows = store.table::<NamedRow>();
	rows.name().xpect_eq("named_rows");
	rows.get("missing").await.unwrap().xpect_none();
	rows.exists("missing").await.unwrap().xpect_false();
	let row = NamedRow::new("one", 7);
	let cid = rows.put(row.clone()).await.unwrap();
	cid.clone()
		.xpect_eq(Row::from_serde(&row).unwrap().cid().unwrap());
	rows.get("one").await.unwrap().xpect_eq(Some(row.clone()));
	rows.get_with_cid("one")
		.await
		.unwrap()
		.xpect_eq(Some((row.clone(), cid.clone())));
	// the erased read answers the cid stored, not one recomputed
	let stored = store.get("named_rows", "one").await.unwrap().unwrap();
	stored.cid().clone().xpect_eq(cid);
	stored.key().as_str().xpect_eq("one");
	// bytes, floats, nesting and an empty map survive
	let rich = Row::from_value(value!({
		"bytes": (Value::Bytes(vec![0, 159, 255])),
		"float": 0.5,
		"negative": (-3i64),
		"nested": { "list": [true, null, "text"], "empty": {} }
	}))
	.unwrap();
	let rich = StoredRow::new("rich", rich).unwrap();
	store
		.put("named_rows", rich.clone(), Precondition::Any)
		.await
		.unwrap();
	let back = store.get("named_rows", "rich").await.unwrap().unwrap();
	back.cid().clone().xpect_eq(rich.cid().clone());
	back.row().cid().unwrap().xpect_eq(rich.cid().clone());
	// tables are disjoint
	store
		.table::<OtherRow>()
		.list()
		.await
		.unwrap()
		.xpect_empty();
	rows.list()
		.await
		.unwrap()
		.xpect_eq(vec![TableKey::from("one"), TableKey::from("rich")]);
	rows.remove("one").await.unwrap();
	rows.remove("rich").await.unwrap();
	// removing a missing row under `Any` succeeds
	rows.remove("one").await.unwrap();
	rows.get("one").await.unwrap().xpect_none();
}

async fn preconditions(store: &TableStore) {
	let rows = store.table::<NamedRow>();
	// absent on an existing row
	let first = rows.push(NamedRow::new("k", 1)).await.unwrap();
	rows.push(NamedRow::new("k", 2)).await.xpect_conflict();
	let err = rows.push(NamedRow::new("k", 2)).await.unwrap_err();
	match TableError::of(&err) {
		Some(TableError::Conflict {
			table, key, found, ..
		}) => {
			table.as_str().xpect_eq("named_rows");
			key.as_str().xpect_eq("k");
			found.clone().xpect_eq(Some(first.clone()));
		}
		_ => panic!("expected a conflict, found {err}"),
	}
	rows.get("k").await.unwrap().unwrap().value.xpect_eq(1);
	// matches the current row
	let second = rows
		.put_if(NamedRow::new("k", 2), Precondition::Matches(first.clone()))
		.await
		.unwrap();
	// matches a changed row
	rows.put_if(NamedRow::new("k", 3), Precondition::Matches(first.clone()))
		.await
		.xpect_conflict();
	rows.get("k").await.unwrap().unwrap().value.xpect_eq(2);
	// matches a missing row
	rows.put_if(
		NamedRow::new("gone", 1),
		Precondition::Matches(first.clone()),
	)
	.await
	.xpect_conflict();
	rows.exists("gone").await.unwrap().xpect_false();
	// any
	rows.put(NamedRow::new("k", 4)).await.unwrap();
	// removes
	rows.remove_if("k", Precondition::Matches(second))
		.await
		.xpect_conflict();
	rows.remove_if("k", Precondition::Absent)
		.await
		.xpect_conflict();
	let current = rows.get_with_cid("k").await.unwrap().unwrap().1;
	rows.remove_if("k", Precondition::Matches(current))
		.await
		.unwrap();
	rows.remove_if("k", Precondition::Absent).await.unwrap();
	rows.remove_if("k", Precondition::Matches(first))
		.await
		.xpect_conflict();
}

/// Many writers each read, increment and conditionally write one counter,
/// retrying on conflict: under conditional writes no increment is lost.
async fn concurrent_writes(store: &TableStore) {
	let rows = store.table::<NamedRow>();
	rows.put(NamedRow::new("counter", 0)).await.unwrap();
	let writers = 8;
	let increments = 3;
	(0..writers)
		.map(|_| {
			let rows = rows.clone();
			async move {
				for _ in 0..increments {
					loop {
						let (mut row, cid) =
							rows.get_with_cid("counter").await?.unwrap();
						row.value += 1;
						// yield between the read and the write, so writers
						// interleave even on a single threaded executor
						async_ext::yield_now().await;
						match rows.put_if(row, Precondition::Matches(cid)).await
						{
							Ok(_) => break,
							Err(err) if TableError::is_conflict(&err) => {}
							Err(err) => return Err(err),
						}
					}
				}
				Ok::<_, BevyError>(())
			}
		})
		.xmap(async_ext::try_join_all)
		.await
		.unwrap();
	rows.get("counter")
		.await
		.unwrap()
		.unwrap()
		.value
		.xpect_eq(writers * increments);
	rows.remove("counter").await.unwrap();
}

async fn scans(store: &TableStore) {
	let rows = store.table::<NamedRow>();
	let keys = ordered_keys();
	// written out of order
	for (idx, key) in keys.iter().enumerate().rev() {
		rows.put(NamedRow::new(*key, idx as i64)).await.unwrap();
	}
	let key_of = |rows: Vec<NamedRow>| {
		rows.into_iter().map(|row| row.key).collect::<Vec<_>>()
	};
	// the whole table in byte order, either direction
	key_of(rows.get_all().await.unwrap()).xpect_eq(keys.clone());
	let mut reversed = keys.clone();
	reversed.reverse();
	key_of(rows.scan_all(Scan::default().reverse()).await.unwrap())
		.xpect_eq(reversed.clone());
	// bounds
	let between = |from: &str, to: &str| {
		keys.iter()
			.filter(|key| {
				key.as_bytes() >= from.as_bytes()
					&& key.as_bytes() <= to.as_bytes()
			})
			.copied()
			.collect::<Vec<_>>()
	};
	key_of(
		rows.scan_all(Scan::default().from("B").through("b"))
			.await
			.unwrap(),
	)
	.xpect_eq(between("B", "b"));
	key_of(
		rows.scan_all(Scan::default().after("B").before("b"))
			.await
			.unwrap(),
	)
	.xpect_eq(between("Z", "aa"));
	key_of(
		rows.scan_all(Scan::default().from("a").through("é").reverse())
			.await
			.unwrap(),
	)
	.xpect_eq(between("a", "é").into_iter().rev().collect::<Vec<_>>());
	// pages across their boundaries, either direction
	for limit in [1, 2, 4, 7] {
		for reverse in [false, true] {
			let mut scan = Scan::default().limit(limit);
			if reverse {
				scan = scan.reverse();
			}
			let mut seen = Vec::new();
			loop {
				let page = rows.scan(scan.clone()).await.unwrap();
				(page.rows.len() <= limit).xpect_true();
				seen.extend(key_of(page.rows));
				match page.cursor {
					Some(cursor) => scan = scan.with_cursor(Some(cursor)),
					None => break,
				}
			}
			seen.xpect_eq(match reverse {
				false => keys.clone(),
				true => reversed.clone(),
			});
		}
	}
	// a bounded, paged scan
	let mut scan = Scan::default().from("A").before("é").limit(3);
	let mut seen = Vec::new();
	loop {
		let page = rows.scan(scan.clone()).await.unwrap();
		seen.extend(key_of(page.rows));
		match page.cursor {
			Some(cursor) => scan = scan.with_cursor(Some(cursor)),
			None => break,
		}
	}
	seen.xpect_eq(
		keys.iter()
			.filter(|key| {
				key.as_bytes() >= "A".as_bytes()
					&& key.as_bytes() < "é".as_bytes()
			})
			.copied()
			.collect::<Vec<_>>(),
	);
	// the first few in either direction, paging only as far as they reach
	key_of(
		rows.scan_first(Scan::default().reverse().before("b"), 3)
			.await
			.unwrap(),
	)
	.xpect_eq(vec!["aa", "a", "_"]);
	key_of(rows.scan_first(Scan::default(), 2).await.unwrap())
		.xpect_eq(keys[..2].to_vec());
	// bounds holding no key answer none
	rows.scan_all(Scan::default().from("b").before("b"))
		.await
		.unwrap()
		.xpect_empty();
	rows.scan_all(Scan::default().from("b").through("B"))
		.await
		.unwrap()
		.xpect_empty();
	for key in keys {
		rows.remove(key).await.unwrap();
	}
	rows.list().await.unwrap().xpect_empty();
}

async fn partitions(store: &TableStore) {
	let root = store.table::<NamedRow>();
	let left = store.partition("left").unwrap();
	let right = store.partition("did:plc:right").unwrap();
	let child = left.partition("child").unwrap();
	left.table::<NamedRow>()
		.put(NamedRow::new("x", 1))
		.await
		.unwrap();
	right
		.table::<NamedRow>()
		.put(NamedRow::new("x", 2))
		.await
		.unwrap();
	child
		.table::<NamedRow>()
		.put(NamedRow::new("x", 3))
		.await
		.unwrap();
	root.get("x").await.unwrap().xpect_none();
	left.table::<NamedRow>()
		.get("x")
		.await
		.unwrap()
		.unwrap()
		.value
		.xpect_eq(1);
	right
		.table::<NamedRow>()
		.get("x")
		.await
		.unwrap()
		.unwrap()
		.value
		.xpect_eq(2);
	child
		.table::<NamedRow>()
		.get("x")
		.await
		.unwrap()
		.unwrap()
		.value
		.xpect_eq(3);
	left.table::<NamedRow>()
		.list()
		.await
		.unwrap()
		.xpect_eq(vec![TableKey::from("x")]);
	// a partition named like a root table is still its own
	let named = store.partition("named_rows").unwrap();
	named
		.table::<NamedRow>()
		.put(NamedRow::new("y", 4))
		.await
		.unwrap();
	root.list().await.unwrap().xpect_empty();
	named.table::<NamedRow>().remove("y").await.unwrap();
	for partition in [&left, &right, &child] {
		partition.table::<NamedRow>().remove("x").await.unwrap();
	}
	store.partition("a b").unwrap_err();
	store.partition("").unwrap_err();
	store.partition("a/../b").unwrap_err();
}

async fn records(store: &TableStore) {
	let notes = store.table::<Rkeyed<Note>>();
	notes.name().xpect_eq("com.example.note");
	let note = Rkeyed::new(Rkey::parse("3mw72aaeuj22n").unwrap(), Note {
		text: "hi".into(),
		scale: 0.5,
	});
	// a record written through a table answers its record's cid
	let cid = notes.put(note.clone()).await.unwrap();
	let record = AtprotoValue::from_serde(&*note)
		.unwrap()
		.into_record(&Note::COLLECTION)
		.unwrap();
	cid.xpect_eq(record.cid());
	notes
		.get(note.rkey())
		.await
		.unwrap()
		.xpect_eq(Some(note.clone()));
	// the float crossed as the data model's float object
	store
		.get("com.example.note", note.rkey())
		.await
		.unwrap()
		.unwrap()
		.row()
		.field("scale.$type")
		.cloned()
		.xpect_eq(Some(Value::from("org.beet.core#float")));
	// the typed path refuses a body naming another collection
	store
		.table::<Rkeyed<Mislabeled>>()
		.put(Rkeyed::new(Rkey::parse("x").unwrap(), Mislabeled {
			r#type: "com.example.other".into(),
		}))
		.await
		.unwrap_err();
	// the erased path refuses what is not a record of its collection
	let erased = |key: &str, row: Value| {
		StoredRow::new(key, Row::from_value(row).unwrap()).unwrap()
	};
	for (key, row, reason) in [
		(
			"x",
			value!({ "$type": "com.example.note", "scale": 0.5 }),
			"no floats",
		),
		("x", value!({ "text": "hi" }), "no `$type`"),
		(
			"x",
			value!({ "$type": "com.example.other" }),
			"declares `$type",
		),
		("a~b c", value!({ "$type": "com.example.note" }), "rkey"),
	] {
		store
			.put("com.example.note", erased(key, row), Precondition::Any)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains(reason);
	}
	// and the same through a batch
	store
		.apply(vec![TableWrite::Put {
			table: "com.example.note".into(),
			row: erased("x", value!({ "text": "hi" })),
			precondition: Precondition::Any,
		}])
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("no `$type`");
	// a valid erased record is stored
	store
		.put(
			"com.example.note",
			erased(
				"valid",
				value!({ "$type": "com.example.note", "text": "hi" }),
			),
			Precondition::Any,
		)
		.await
		.unwrap();
	notes.remove("valid").await.unwrap();
	notes.remove(note.rkey()).await.unwrap();
}

async fn atomic_apply(store: &TableStore) {
	let rows = store.table::<NamedRow>();
	let others = store.table::<OtherRow>();
	let batch = |precondition: Precondition| {
		vec![
			rows.put_write(NamedRow::new("a", 1), Precondition::Absent)
				.unwrap(),
			others
				.put_write(OtherRow { key: "b".into() }, Precondition::Any)
				.unwrap(),
			rows.put_write(NamedRow::new("c", 3), precondition).unwrap(),
		]
	};
	if !store.supports(Capability::AtomicApply) {
		xpect_unsupported(
			store.apply(batch(Precondition::Any)).await,
			Capability::AtomicApply,
		);
		rows.list().await.unwrap().xpect_empty();
		return;
	}
	// a precondition failing midway lands nothing
	store
		.apply(batch(Precondition::Matches(Cid::raw(b"nope"))))
		.await
		.xpect_conflict();
	rows.list().await.unwrap().xpect_empty();
	others.list().await.unwrap().xpect_empty();
	// a batch whose preconditions all hold lands whole, across tables
	store.apply(batch(Precondition::Absent)).await.unwrap();
	rows.list().await.unwrap().len().xpect_eq(2);
	others.get("b").await.unwrap().xpect_some();
	// removes batch too
	let cid = rows.get_with_cid("a").await.unwrap().unwrap().1;
	store
		.apply(vec![
			rows.remove_write("a", Precondition::Matches(cid)),
			rows.remove_write("c", Precondition::Any),
			others.remove_write("b", Precondition::Any),
		])
		.await
		.unwrap();
	rows.list().await.unwrap().xpect_empty();
	others.list().await.unwrap().xpect_empty();
}

async fn indexes(store: &TableStore) {
	let rows = store.table::<IndexedRow>();
	if !store.supports(Capability::Indexes) {
		xpect_unsupported(
			rows.scan_index("by_rank", IndexScan::default()).await,
			Capability::Indexes,
		);
		return;
	}
	// one row per value kind, so the backend's order is checked against the
	// encoding's for each
	let values = ordered_values();
	for (idx, group) in values.iter().enumerate().rev() {
		rows.put(IndexedRow {
			key: format!("k{idx:02}"),
			group: group.clone(),
			rank: (idx % 3) as i64,
		})
		.await
		.unwrap();
	}
	let keys = |rows: Vec<IndexedRow>| {
		rows.into_iter().map(|row| row.key).collect::<Vec<_>>()
	};
	let all = |scan: IndexScan| {
		let rows = rows.clone();
		async move {
			let mut scan = scan;
			let mut seen = Vec::new();
			loop {
				let page =
					rows.scan_index("by_group", scan.clone()).await.unwrap();
				seen.extend(page.rows);
				match page.cursor {
					Some(cursor) => scan = scan.with_cursor(Some(cursor)),
					None => break,
				}
			}
			seen
		}
	};
	let expected = (0..values.len())
		.map(|idx| format!("k{idx:02}"))
		.collect::<Vec<_>>();
	keys(all(IndexScan::default()).await).xpect_eq(expected.clone());
	keys(all(IndexScan::default().limit(5)).await).xpect_eq(expected.clone());
	keys(all(IndexScan::default().limit(4).reverse()).await)
		.xpect_eq(expected.iter().rev().cloned().collect::<Vec<_>>());
	// a prefix is a range
	keys(all(IndexScan::default().eq([Value::Str("a".into())])).await)
		.xpect_eq(vec![format!(
			"k{:02}",
			values
				.iter()
				.position(|value| *value == Value::Str("a".into()))
				.unwrap()
		)]);
	// ranks tie, so rows order by rank then key
	let by_rank = rows
		.scan_index("by_rank", IndexScan::default().eq([1i64]))
		.await
		.unwrap();
	keys(by_rank.rows).xpect_eq(
		(0..values.len())
			.filter(|idx| idx % 3 == 1)
			.map(|idx| format!("k{idx:02}"))
			.collect::<Vec<_>>(),
	);
	let lowest = rows
		.scan_index(
			"by_rank",
			IndexScan::default().after([0i64]).before([2i64]),
		)
		.await
		.unwrap();
	lowest.rows.iter().all(|row| row.rank == 1).xpect_true();
	// an update moves a row within the index, a remove drops it
	let first = rows.get("k00").await.unwrap().unwrap();
	rows.put(IndexedRow { rank: 2, ..first }).await.unwrap();
	rows.scan_index("by_rank", IndexScan::default().eq([2i64]))
		.await
		.unwrap()
		.rows
		.iter()
		.any(|row| row.key == "k00")
		.xpect_true();
	for idx in 0..values.len() {
		rows.remove(format!("k{idx:02}")).await.unwrap();
	}
	rows.scan_index("by_group", IndexScan::default())
		.await
		.unwrap()
		.rows
		.xpect_empty();
	// an indexed field holding a list is refused
	rows.put(IndexedRow {
		key: "list".into(),
		group: value!([1]),
		rank: 0,
	})
	.await
	.unwrap_err();
}

/// Expired rows are gone from every read, and a write treats them as absent.
async fn expiry(store: &TableStore) {
	let rows = store.table::<NamedRow>();
	for key in ["a", "b", "c"] {
		rows.put(NamedRow::new(key, 1)).await.unwrap();
	}
	store
		.expire("named_rows", vec!["a".into(), "b".into(), "missing".into()])
		.await
		.unwrap();
	rows.get("a").await.unwrap().xpect_none();
	rows.list()
		.await
		.unwrap()
		.xpect_eq(vec![TableKey::from("c")]);
	rows.push(NamedRow::new("a", 2)).await.unwrap();
	rows.get("a").await.unwrap().unwrap().value.xpect_eq(2);
	for key in ["a", "c"] {
		rows.remove(key).await.unwrap();
	}
}
