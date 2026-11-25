//! The SQL every SQLite table backend runs, whatever engine runs it: a file
//! through rusqlite ([`SqliteStore`]) or a Durable Object's own storage
//! (`DurableObjectStore`). One schema and one set of semantics, so a
//! partition behaves identically in either and moves between them by
//! declaration.
use crate::prelude::*;
use beet_core::prelude::*;
use core::ops::Bound;

/// One SQL value, bound as a parameter or read from a column: SQLite's five
/// storage classes.
#[derive(Debug, Clone, PartialEq)]
pub enum SqlArg {
	/// `NULL`.
	Null,
	/// An integer.
	Integer(i64),
	/// A float.
	Real(f64),
	/// Text.
	Text(String),
	/// A blob.
	Blob(Vec<u8>),
}

impl SqlArg {
	/// The text this holds, refused for any other class.
	pub fn into_text(self) -> Result<String> {
		match self {
			Self::Text(text) => text.xok(),
			other => bevybail!("expected a text column, found {other:?}"),
		}
	}
}

/// What runs [`SqlTable`]'s statements: an open SQLite database.
pub trait SqlEngine {
	/// Run `sql` with `args`, answering the rows it changed.
	fn execute(&self, sql: &str, args: &[SqlArg]) -> Result<usize>;
	/// Run `sql` with `args`, answering every row's columns.
	fn query(&self, sql: &str, args: &[SqlArg]) -> Result<Vec<Vec<SqlArg>>>;
	/// Run `func` as one transaction: every statement it runs commits
	/// together, or none does when it errors.
	fn transaction(
		&self,
		func: &mut dyn FnMut(&dyn SqlEngine) -> Result,
	) -> Result;
}

/// One row table's SQL: its validated name, quoted identifier, and the
/// expressions each declared index orders by.
///
/// A table is `key TEXT PRIMARY KEY, json TEXT NOT NULL, cid TEXT NOT NULL`
/// under SQLite's default `BINARY` collation, which is byte order. Every
/// precondition is one statement checked by its changed row count, and each
/// declared index is an expression index over `json_extract` of its fields
/// then the key, whose order matches [`IndexKey`]'s. A table that does not
/// exist yet reads as an empty one.
pub struct SqlTable {
	/// The table name, ie `named_rows` or `com.example.note`.
	name: SmolStr,
	/// The quoted identifier.
	ident: String,
	/// Each declared index: its identifier and its fields' expressions.
	indexes: Vec<(TableIndex, String, Vec<String>)>,
}

impl SqlTable {
	/// Validate `spec`'s names so none reaches the SQL text unchecked: a
	/// table is snake case (`[a-z][a-z0-9_]*`) or an NSID, never `blobs`; an
	/// index is snake case, a field dotted segments free of quotes.
	pub fn new(spec: &TableSpec) -> Result<Self> {
		let name = spec.name.clone();
		if !(Self::is_snake(&name) || spec.collection().is_some()) {
			bevybail!(
				"invalid sql table name `{name}`: expected [a-z][a-z0-9_]* or an NSID"
			);
		}
		if name == "blobs" {
			bevybail!("`blobs` is the sql blob table, not a row table");
		}
		let indexes = spec
			.indexes
			.iter()
			.map(|index| {
				if !Self::is_snake(&index.name) {
					bevybail!(
						"invalid index name `{}`: expected [a-z][a-z0-9_]*",
						index.name
					);
				}
				let exprs = index
					.fields
					.iter()
					.map(|field| Self::field_expr(field))
					.collect::<Result<Vec<_>>>()?;
				Ok((
					index.clone(),
					format!("\"{name}__{}\"", index.name),
					exprs,
				))
			})
			.collect::<Result<Vec<_>>>()?;
		Self {
			ident: format!("\"{name}\""),
			name,
			indexes,
		}
		.xok()
	}

	fn is_snake(name: &str) -> bool {
		name.chars()
			.next()
			.is_some_and(|first| first.is_ascii_lowercase())
			&& name.chars().all(|ch| {
				ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_'
			})
	}

	/// The `json_extract` of a dotted field path, each segment quoted.
	fn field_expr(field: &str) -> Result<String> {
		let path = field
			.split('.')
			.map(|segment| {
				if segment.is_empty()
					|| segment.contains(['"', '\'', '\\'])
					|| segment.chars().any(char::is_control)
				{
					bevybail!("invalid index field `{field}`");
				}
				format!(".\"{segment}\"").xok()
			})
			.collect::<Result<String>>()?;
		format!("json_extract(json, '${path}')").xok()
	}

	/// Whether the table exists yet.
	pub fn exists(&self, sql: &dyn SqlEngine) -> Result<bool> {
		sql.query(
			"SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
			&[SqlArg::Text(self.name.to_string())],
		)?
		.is_empty()
		.xmap(|empty| !empty)
		.xok()
	}

	/// Create the table and its indexes if missing, for a write.
	pub fn ensure(&self, sql: &dyn SqlEngine) -> Result {
		sql.execute(
			&format!(
				"CREATE TABLE IF NOT EXISTS {} (\
					key TEXT PRIMARY KEY, json TEXT NOT NULL, cid TEXT NOT NULL)",
				self.ident
			),
			&[],
		)?;
		for (_, ident, exprs) in &self.indexes {
			sql.execute(
				&format!(
					"CREATE INDEX IF NOT EXISTS {ident} ON {} ({}, key)",
					self.ident,
					exprs.join(", ")
				),
				&[],
			)?;
		}
		Ok(())
	}

	/// The stored row its `(key, json, cid)` columns hold, its cid as stored.
	fn stored_row(columns: Vec<SqlArg>) -> Result<StoredRow> {
		let [key, json, cid]: [SqlArg; 3] = columns
			.try_into()
			.map_err(|_| bevyhow!("expected the key, json and cid columns"))?;
		StoredRow::from_parts(
			key.into_text()?.into(),
			Cid::parse(&cid.into_text()?)?,
			Row::parse_json(&json.into_text()?)?,
		)
		.xok()
	}

	/// The row at `key`, `None` where there is none.
	pub fn get(
		&self,
		sql: &dyn SqlEngine,
		key: &TableKey,
	) -> Result<Option<StoredRow>> {
		if !self.exists(sql)? {
			return Ok(None);
		}
		sql.query(
			&format!(
				"SELECT key, json, cid FROM {} WHERE key = ?1",
				self.ident
			),
			&[SqlArg::Text(key.to_string())],
		)?
		.into_iter()
		.next()
		.map(Self::stored_row)
		.transpose()
	}

	/// The cid of the row at `key`, `None` where there is none.
	fn current_cid(
		&self,
		sql: &dyn SqlEngine,
		key: &TableKey,
	) -> Result<Option<Cid>> {
		self.get(sql, key)?.map(|row| row.cid().clone()).xok()
	}

	/// The conflict at `key`, naming the row there now.
	fn conflict(
		&self,
		sql: &dyn SqlEngine,
		key: &TableKey,
		precondition: &Precondition,
	) -> BevyError {
		match self.current_cid(sql, key) {
			Ok(found) => TableError::Conflict {
				table: self.name.clone(),
				key: key.clone(),
				expected: precondition.clone(),
				found,
			}
			.into(),
			Err(err) => err,
		}
	}

	/// Write `row` under `precondition` in one statement, its changed row
	/// count saying whether the precondition held. Creates the table first.
	pub fn put(
		&self,
		sql: &dyn SqlEngine,
		row: &StoredRow,
		precondition: &Precondition,
	) -> Result {
		// refuse an unindexable value before anything is written
		for (index, ..) in &self.indexes {
			index.values(row.row())?;
		}
		self.ensure(sql)?;
		let args = [
			SqlArg::Text(row.key().to_string()),
			SqlArg::Text(serde_json::to_string(&row.row().to_json())?),
			SqlArg::Text(row.cid().to_string()),
		];
		let ident = &self.ident;
		let changed = match precondition {
			Precondition::Any => sql.execute(
				&format!(
					"INSERT INTO {ident} (key, json, cid) VALUES (?1, ?2, ?3) \
					 ON CONFLICT(key) DO UPDATE SET json = excluded.json, cid = excluded.cid"
				),
				&args,
			)?,
			Precondition::Absent => sql.execute(
				&format!(
					"INSERT INTO {ident} (key, json, cid) VALUES (?1, ?2, ?3) \
					 ON CONFLICT(key) DO NOTHING"
				),
				&args,
			)?,
			Precondition::Matches(expected) => sql.execute(
				&format!(
					"UPDATE {ident} SET json = ?2, cid = ?3 WHERE key = ?1 AND cid = ?4"
				),
				&[args.as_slice(), &[SqlArg::Text(expected.to_string())]]
					.concat(),
			)?,
		};
		match changed {
			0 => Err(self.conflict(sql, row.key(), precondition)),
			_ => Ok(()),
		}
	}

	/// Remove `key` under `precondition`.
	pub fn remove(
		&self,
		sql: &dyn SqlEngine,
		key: &TableKey,
		precondition: &Precondition,
	) -> Result {
		if !self.exists(sql)? {
			return precondition
				.check(&self.name, key, None)
				.map_err(BevyError::from);
		}
		let ident = &self.ident;
		match precondition {
			Precondition::Any => {
				sql.execute(
					&format!("DELETE FROM {ident} WHERE key = ?1"),
					&[SqlArg::Text(key.to_string())],
				)?;
				Ok(())
			}
			Precondition::Absent => precondition
				.check(&self.name, key, self.current_cid(sql, key)?.as_ref())
				.map_err(BevyError::from),
			Precondition::Matches(expected) => match sql.execute(
				&format!("DELETE FROM {ident} WHERE key = ?1 AND cid = ?2"),
				&[
					SqlArg::Text(key.to_string()),
					SqlArg::Text(expected.to_string()),
				],
			)? {
				0 => Err(self.conflict(sql, key, precondition)),
				_ => Ok(()),
			},
		}
	}

	/// Every write or none: each precondition is checked first, against the
	/// rows as the earlier writes of the batch leave them, and only once all
	/// hold are the writes made, in one transaction. A failed precondition
	/// therefore writes nothing on any engine, a Durable Object's included,
	/// whose storage refuses `BEGIN` and whose events never interleave.
	pub fn apply(sql: &dyn SqlEngine, writes: &[TableWrite]) -> Result {
		let tables = writes
			.iter()
			.map(|write| Self::new(write.table()))
			.collect::<Result<Vec<_>>>()?;
		let mut after = HashMap::<(SmolStr, TableKey), Option<Cid>>::default();
		for (table, write) in tables.iter().zip(writes) {
			let slot = (table.name.clone(), write.key().clone());
			let current = match after.get(&slot) {
				Some(current) => current.clone(),
				None => table.current_cid(sql, write.key())?,
			};
			write.precondition().check(
				&table.name,
				write.key(),
				current.as_ref(),
			)?;
			let next = match write {
				TableWrite::Put { row, .. } => {
					for (index, ..) in &table.indexes {
						index.values(row.row())?;
					}
					Some(row.cid().clone())
				}
				TableWrite::Remove { .. } => None,
			};
			after.insert(slot, next);
		}
		sql.transaction(&mut |sql| {
			for (table, write) in tables.iter().zip(writes) {
				match write {
					TableWrite::Put { row, .. } => {
						table.put(sql, row, &Precondition::Any)?
					}
					TableWrite::Remove { key, .. } => {
						table.remove(sql, key, &Precondition::Any)?
					}
				}
			}
			Ok(())
		})
	}

	/// One page in key order.
	pub fn scan(&self, sql: &dyn SqlEngine, scan: &Scan) -> Result<ScanPage> {
		if !self.exists(sql)? {
			return Ok(ScanPage::default());
		}
		let scan = scan.after_key_cursor();
		let mut conds = Vec::new();
		let mut args = Vec::<SqlArg>::new();
		let mut bound = |op: &str, key: &TableKey| {
			args.push(SqlArg::Text(key.to_string()));
			conds.push(format!("key {op} ?{}", args.len()));
		};
		match &scan.start {
			Bound::Included(key) => bound(">=", key),
			Bound::Excluded(key) => bound(">", key),
			Bound::Unbounded => {}
		}
		match &scan.end {
			Bound::Included(key) => bound("<=", key),
			Bound::Excluded(key) => bound("<", key),
			Bound::Unbounded => {}
		}
		let order = match scan.reverse {
			true => "key DESC",
			false => "key ASC",
		};
		let rows = self.select(sql, &conds, &args, order, scan.limit)?;
		ScanPage::from_sorted(
			match scan.reverse {
				// `from_sorted` takes ascending rows and reverses them itself
				true => rows.into_iter().rev().collect(),
				false => rows,
			},
			&scan,
		)
		.xok()
	}

	/// One page in the order of `index`, each bound a null-aware
	/// lexicographic comparison over its leading fields, so a bound or cursor
	/// holding null compares as SQLite orders it, lowest.
	pub fn scan_index(
		&self,
		sql: &dyn SqlEngine,
		index: &TableIndex,
		scan: &IndexScan,
	) -> Result<ScanPage> {
		if !self.exists(sql)? {
			return Ok(ScanPage::default());
		}
		let Some((_, _, exprs)) =
			self.indexes.iter().find(|(declared, ..)| declared == index)
		else {
			bevybail!(
				"table `{}` declares no index `{}`",
				self.name,
				index.name
			);
		};
		let mut conds = Vec::new();
		let mut args = Vec::<SqlArg>::new();
		let mut tuple = |values: &[Value],
		                 with_key: bool,
		                 op: TupleOp|
		 -> Result {
			let mut columns =
				exprs.iter().take(values.len()).cloned().collect::<Vec<_>>();
			if with_key {
				columns.push("key".into());
			}
			conds.push(op.sql(&columns, values, &mut args)?);
			Ok(())
		};
		match &scan.start {
			Bound::Included(values) => tuple(values, false, TupleOp::Ge)?,
			Bound::Excluded(values) => tuple(values, false, TupleOp::Gt)?,
			Bound::Unbounded => {}
		}
		match &scan.end {
			Bound::Included(values) => tuple(values, false, TupleOp::Le)?,
			Bound::Excluded(values) => tuple(values, false, TupleOp::Lt)?,
			Bound::Unbounded => {}
		}
		if let Some(cursor) = &scan.cursor {
			// the entry a page stopped at: its fields then its key
			let values = IndexKey::from_cursor(cursor)?.decode()?;
			if values.len() != exprs.len() + 1 {
				bevybail!("cursor is not an entry of index `{}`", index.name);
			}
			tuple(&values, true, match scan.reverse {
				false => TupleOp::Gt,
				true => TupleOp::Lt,
			})?;
		}
		let direction = match scan.reverse {
			true => "DESC",
			false => "ASC",
		};
		let order = exprs
			.iter()
			.map(String::as_str)
			.chain(["key"])
			.map(|expr| format!("{expr} {direction}"))
			.collect::<Vec<_>>()
			.join(", ");
		let mut rows = self.select(sql, &conds, &args, &order, scan.limit)?;
		let more = scan.limit.is_some_and(|limit| rows.len() > limit);
		if let Some(limit) = scan.limit {
			rows.truncate(limit);
		}
		let cursor = match (more, rows.last()) {
			(true, Some(last)) => {
				Some(index.entry(last.key(), last.row())?.to_cursor())
			}
			_ => None,
		};
		ScanPage { rows, cursor }.xok()
	}

	/// Select rows matching every condition in `order`, one past `limit` so
	/// the caller knows whether another page follows.
	fn select(
		&self,
		sql: &dyn SqlEngine,
		conds: &[String],
		args: &[SqlArg],
		order: &str,
		limit: Option<usize>,
	) -> Result<Vec<StoredRow>> {
		let filter = match conds.is_empty() {
			true => String::new(),
			false => format!("WHERE {}", conds.join(" AND ")),
		};
		let limit = limit
			.map(|limit| format!("LIMIT {}", limit + 1))
			.unwrap_or_default();
		sql.query(
			&format!(
				"SELECT key, json, cid FROM {} {filter} ORDER BY {order} {limit}",
				self.ident
			),
			args,
		)?
		.into_iter()
		.map(Self::stored_row)
		.collect()
	}
}

/// A lexicographic comparison of SQL columns against a tuple of values.
#[derive(Clone, Copy)]
enum TupleOp {
	Gt,
	Ge,
	Lt,
	Le,
}

impl TupleOp {
	/// The null-aware condition `columns op values`, binding each value once
	/// into `args`: for `>`, some leading run of columns equals the values
	/// (`IS`, so null equals null) and the next column is greater, null
	/// ordering lowest.
	fn sql(
		self,
		columns: &[String],
		values: &[Value],
		args: &mut Vec<SqlArg>,
	) -> Result<String> {
		let params = values
			.iter()
			.map(|value| match Self::literal(value) {
				Some(literal) => literal.xok(),
				None => {
					args.push(Self::bind(value)?);
					format!("?{}", args.len()).xok()
				}
			})
			.collect::<Result<Vec<_>>>()?;
		let pairs = columns.iter().zip(params.iter()).collect::<Vec<_>>();
		let mut clauses = (0..pairs.len())
			.map(|idx| {
				let equal = pairs[..idx]
					.iter()
					.map(|(column, param)| format!("{column} IS {param}"));
				let (column, param) = pairs[idx];
				let step = match self {
					Self::Gt | Self::Ge => format!(
						"({column} IS NOT NULL AND ({param} IS NULL OR {column} > {param}))"
					),
					Self::Lt | Self::Le => format!(
						"({param} IS NOT NULL AND ({column} IS NULL OR {column} < {param}))"
					),
				};
				format!(
					"({})",
					equal
						.chain(core::iter::once(step))
						.collect::<Vec<_>>()
						.join(" AND ")
				)
			})
			.collect::<Vec<_>>();
		if matches!(self, Self::Ge | Self::Le) {
			clauses.push(format!(
				"({})",
				pairs
					.iter()
					.map(|(column, param)| format!("{column} IS {param}"))
					.collect::<Vec<_>>()
					.join(" AND ")
			));
		}
		match clauses.is_empty() {
			// an empty tuple bounds nothing
			true => "1".to_string(),
			false => format!("({})", clauses.join(" OR ")),
		}
		.xok()
	}

	/// An integer (a bool as one) as a literal in the statement rather than a
	/// parameter: a Durable Object binds through a js number, which holds an
	/// integer past 2^53 only approximately, and a literal is exact on every
	/// engine. Digits alone, so nothing reaches the SQL text unchecked.
	fn literal(value: &Value) -> Option<String> {
		match value {
			Value::Bool(bool) => Some((*bool as i64).to_string()),
			Value::Int(int) => Some(int.to_string()),
			Value::Uint(uint) if *uint <= i64::MAX as u64 => {
				Some(uint.to_string())
			}
			_ => None,
		}
	}

	/// A scalar as the SQL value `json_extract` would answer for it.
	fn bind(value: &Value) -> Result<SqlArg> {
		IndexKey::check_scalar(value)?;
		match value {
			Value::Null => SqlArg::Null,
			Value::Bool(bool) => SqlArg::Integer(*bool as i64),
			Value::Int(int) => SqlArg::Integer(*int),
			Value::Uint(uint) => match i64::try_from(*uint) {
				Ok(int) => SqlArg::Integer(int),
				Err(_) => SqlArg::Real(*uint as f64),
			},
			Value::Float(float) => SqlArg::Real(*float),
			Value::Str(string) => SqlArg::Text(string.to_string()),
			_ => unreachable!("checked scalar"),
		}
		.xok()
	}
}
