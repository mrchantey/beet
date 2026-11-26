//! [`PartitionRows`]: one partition's rows as plain maps, the logic a table
//! store held in one piece (a json file, process memory) shares.
use crate::prelude::*;
use beet_core::prelude::*;
use std::collections::BTreeMap;

/// One partition's tables, each its rows in key order: reads, preconditions,
/// writes and scans over rows a store holds whole.
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct PartitionRows(
	pub(crate) BTreeMap<SmolStr, BTreeMap<TableKey, Row>>,
);

impl PartitionRows {
	pub(crate) fn is_empty(&self) -> bool { self.0.is_empty() }

	/// The rows of `table`, in key order.
	fn rows(
		&self,
		table: &TableSpec,
	) -> impl Iterator<Item = (&TableKey, &Row)> {
		self.0.get(&table.name).into_iter().flatten()
	}

	/// The row at `key`, its cid computed here.
	pub(crate) fn get(
		&self,
		table: &TableSpec,
		key: &TableKey,
	) -> Result<Option<StoredRow>> {
		self.0
			.get(&table.name)
			.and_then(|rows| rows.get(key))
			.map(|row| StoredRow::new(key, row.clone()))
			.transpose()
	}

	/// Make one write when its precondition holds, answering whether it
	/// changed anything.
	pub(crate) fn apply(&mut self, write: &TableWrite) -> Result<bool> {
		let (table, key) = (write.table(), write.key());
		// only a precondition on the row reads its cid
		if *write.precondition() != Precondition::Any {
			let found = self.get(table, key)?;
			write.precondition().check(
				&table.name,
				key,
				found.as_ref().map(StoredRow::cid),
			)?;
		}
		match write {
			TableWrite::Put { table, row, .. } => {
				for index in &table.indexes {
					index.values(row.row())?;
				}
				let rows = self.0.entry(table.name.clone()).or_default();
				let before = rows.insert(key.clone(), row.row().clone());
				(before.as_ref() != Some(row.row())).xok()
			}
			TableWrite::Remove { .. } => {
				let Some(rows) = self.0.get_mut(&table.name) else {
					return Ok(false);
				};
				let removed = rows.remove(key).is_some();
				if rows.is_empty() {
					self.0.remove(&table.name);
				}
				removed.xok()
			}
		}
	}

	pub(crate) fn scan(
		&self,
		table: &TableSpec,
		scan: &Scan,
	) -> Result<ScanPage> {
		let scan = scan.after_key_cursor();
		let mut keys = self
			.rows(table)
			.filter(|(key, _)| scan.contains(key))
			.collect::<Vec<_>>();
		if scan.reverse {
			keys.reverse();
		}
		// one past the limit says whether there is another page
		if let Some(limit) = scan.limit {
			keys.truncate(limit + 1);
		}
		let mut rows = keys
			.into_iter()
			.map(|(key, row)| StoredRow::new(key, row.clone()))
			.collect::<Result<Vec<_>>>()?;
		if scan.reverse {
			rows.reverse();
		}
		ScanPage::from_sorted(rows, &scan).xok()
	}

	pub(crate) fn scan_index(
		&self,
		table: &TableSpec,
		index: &TableIndex,
		scan: &IndexScan,
	) -> Result<ScanPage> {
		let range = scan.entry_range()?;
		let mut entries = self
			.rows(table)
			.map(|(key, row)| (index.entry(key, row)?, key, row).xok())
			.collect::<Result<Vec<_>>>()?;
		entries.retain(|(entry, ..)| IndexScan::contains(&range, entry));
		entries.sort_by(|(left, ..), (right, ..)| left.cmp(right));
		if scan.reverse {
			entries.reverse();
		}
		let more = scan.limit.is_some_and(|limit| entries.len() > limit);
		if let Some(limit) = scan.limit {
			entries.truncate(limit);
		}
		ScanPage {
			cursor: match more {
				true => entries.last().map(|(entry, ..)| entry.to_cursor()),
				false => None,
			},
			rows: entries
				.into_iter()
				.map(|(_, key, row)| StoredRow::new(key, row.clone()))
				.collect::<Result<_>>()?,
		}
		.xok()
	}
}
