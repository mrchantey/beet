//! The read-only SQL question a route asks of a declared [`SqliteStore`].
use crate::prelude::*;
use beet_core::prelude::*;

/// Request params for [`SqlSelect`], surfaced in `--help`.
#[derive(Reflect)]
struct SqlSelectParams {
	/// How the rows are written: `table` for aligned columns, `jsonl` for one
	/// json object per line.
	format: Option<RowFormat>,
}

/// How [`SqlSelect`] writes its rows.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Default)]
pub enum RowFormat {
	/// Aligned columns under a header, for a person reading them. Cells wider
	/// than [`SqlSelect::MAX_CELL`] are elided, so a column holding a
	/// megabyte of transcript still lines up.
	#[default]
	Table,
	/// One json object per line, for a pipe. The whole of every cell.
	Jsonl,
}

/// `<Route path="query/senders" {(SqlSelect{sql:".."}, StoreRef($index))}/>`:
/// answer one named question of a declared [`SqliteStore`], the sql written
/// where the route is.
///
/// It reads and never writes, and the absence of a write sibling is
/// deliberate: a SQLite store a consumer queries is an index, whose schema
/// comes from whatever fills it and whose content is rebuilt from that source,
/// so a write here would edit a view of something else.
///
/// The store is named by relation rather than by a path field, so the one
/// declaration answers every question asked of it:
/// `<SqliteStore bx:ref="index" path="data.db"/>` above, `StoreRef($index)`
/// on each route below.
#[action]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
#[require(ParamsPartial = ParamsPartial::new::<SqlSelectParams>())]
pub async fn SqlSelect(
	/// The `SELECT` or `WITH` this route answers.
	#[field(required)]
	sql: String,
	cx: ActionContext<Request>,
) -> Result<Response> {
	let format = cx
		.input
		.parse_params::<SqlSelectParams>()?
		.format
		.unwrap_or_default();
	// the authored half first: a typo fails now rather than after the backoff
	// a missing declaration spends
	SqlSelect::guard(&sql)?;
	let caller = cx.caller.clone();
	let target = caller
		.get::<StoreRef, _>(|store_ref| store_ref.store())
		.await
		.map_err(|_| {
			bevyhow!(
				"a sql query names the database it asks: add \
				 `StoreRef($declaration)` beside it, pointing at the \
				 `<SqliteStore>` that declares the index"
			)
		})?;
	let store =
		StoreRef::resolve::<SqliteStore>(caller.world(), target).await?;
	let rows = store.query(&sql).await?;
	Response::ok_text(SqlSelect::render(&rows, format)).xok()
}

impl SqlSelect {
	/// Characters of one [`RowFormat::Table`] cell before it is elided.
	pub const MAX_CELL: usize = 80;

	/// Refuse anything but a single `SELECT` or `WITH`.
	///
	/// A typo guard and not a security boundary: rusqlite prepares only the
	/// first statement of what it is given, so a trailing `; DROP ..` is
	/// already inert. A read-only connection or a SQLite authorizer is the
	/// upgrade if a query ever comes from anywhere but the document.
	fn guard(sql: &str) -> Result {
		let first = sql
			.split_whitespace()
			.next()
			.ok_or_else(|| bevyhow!("empty sql"))?;
		match first.eq_ignore_ascii_case("select")
			|| first.eq_ignore_ascii_case("with")
		{
			true => Ok(()),
			false => bevybail!(
				"`{first}` is not a read: a sql route answers a single \
				 `SELECT` or `WITH`"
			),
		}
	}

	/// The rows as the text of one response body.
	fn render(rows: &[Value], format: RowFormat) -> String {
		match format {
			RowFormat::Jsonl => Self::jsonl(rows),
			RowFormat::Table => Self::table(rows),
		}
	}

	/// One json object per line, the whole of every cell, keys sorted: the
	/// canonical json every other beet jsonl is written in, so a row reads
	/// the same however it was built.
	fn jsonl(rows: &[Value]) -> String {
		// every line terminated, including the last, so the body concatenates
		// with the next one a pipe reads
		rows.iter()
			.map(|row| format!("{}\n", row.clone().into_json()))
			.collect()
	}

	/// Aligned columns under a header, then the row count. The columns are
	/// the first row's, which every row of one `SELECT` shares.
	fn table(rows: &[Value]) -> String {
		let Some(Value::Map(first)) = rows.first() else {
			return "no rows\n".to_string();
		};
		let columns = first.keys().cloned().collect::<Vec<_>>();
		let cell = |row: &Value, column: &SmolStr| match row {
			Value::Map(map) => {
				map.get(column).map(Self::cell).unwrap_or_default()
			}
			other => Self::cell(other),
		};
		let body = rows
			.iter()
			.map(|row| {
				columns
					.iter()
					.map(|column| cell(row, column))
					.collect::<Vec<_>>()
			})
			.collect::<Vec<_>>();
		let widths = columns
			.iter()
			.enumerate()
			.map(|(idx, column)| {
				body.iter()
					.map(|row| row[idx].chars().count())
					.chain([column.chars().count()])
					.max()
					.unwrap_or_default()
			})
			.collect::<Vec<_>>();
		// a trailing pad is invisible and would make every row's width the
		// table's, so each line is trimmed back to its last cell
		let line = |cells: &[String]| {
			cells
				.iter()
				.zip(widths.iter().copied())
				.map(|(cell, width)| format!("{cell:<width$}"))
				.collect::<Vec<_>>()
				.join("  ")
				.trim_end()
				.to_string()
		};
		let header = columns.iter().map(SmolStr::to_string).collect::<Vec<_>>();
		let rule = widths
			.iter()
			.map(|width| "-".repeat(*width))
			.collect::<Vec<_>>();
		let plural = if rows.len() == 1 { "" } else { "s" };
		[line(&header), line(&rule)]
			.into_iter()
			.chain(body.iter().map(|row| line(row)))
			.chain([format!("({} row{plural})", rows.len())])
			.collect::<Vec<_>>()
			.join("\n")
			+ "\n"
	}

	/// One cell's text: a null reads as nothing, bytes as their size, and
	/// anything wider than [`MAX_CELL`](Self::MAX_CELL) is elided, since a
	/// table is read by a person and the whole of a cell is what
	/// [`RowFormat::Jsonl`] is for.
	fn cell(value: &Value) -> String {
		let text = match value {
			Value::Null => String::new(),
			Value::Bytes(bytes) => format!("<{} bytes>", bytes.len()),
			other => other.to_string().replace(['\n', '\t'], " "),
		};
		match text.chars().count() > Self::MAX_CELL {
			true => {
				text.chars().take(Self::MAX_CELL - 1).collect::<String>()
					+ "\u{2026}"
			}
			false => text,
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	fn rows() -> Vec<Value> {
		vec![
			Value::Map(
				[
					("sender".into(), Value::str("ada")),
					("messages".into(), Value::Int(2)),
				]
				.into_iter()
				.collect(),
			),
			Value::Map(
				[
					("sender".into(), Value::str("grace")),
					("messages".into(), Value::Null),
				]
				.into_iter()
				.collect(),
			),
		]
	}

	#[beet_core::test]
	fn refuses_a_write() {
		SqlSelect::guard("SELECT 1").unwrap();
		SqlSelect::guard("  with x as (select 1) select * from x").unwrap();
		SqlSelect::guard("DROP TABLE facebook_messages")
			.unwrap_err()
			.to_string()
			.xpect_contains("is not a read");
	}

	#[beet_core::test]
	fn aligns_the_columns() {
		SqlSelect::render(&rows(), RowFormat::Table).xpect_eq(
			"sender  messages\n\
			 ------  --------\n\
			 ada     2\n\
			 grace\n\
			 (2 rows)\n",
		);
	}

	#[beet_core::test]
	fn elides_a_wide_cell() {
		let wide = vec![Value::Map(
			[("text".into(), Value::str("x".repeat(200)))]
				.into_iter()
				.collect(),
		)];
		SqlSelect::render(&wide, RowFormat::Table)
			.lines()
			.nth(2)
			.unwrap()
			.chars()
			.count()
			.xpect_eq(SqlSelect::MAX_CELL);
	}

	#[beet_core::test]
	fn writes_jsonl() {
		SqlSelect::render(&rows(), RowFormat::Jsonl).xpect_eq(
			"{\"messages\":2,\"sender\":\"ada\"}\n\
			 {\"messages\":null,\"sender\":\"grace\"}\n",
		);
	}

	#[beet_core::test]
	fn no_rows_is_not_an_empty_table() {
		SqlSelect::render(&[], RowFormat::Table).xpect_eq("no rows\n");
		SqlSelect::render(&[], RowFormat::Jsonl).xpect_eq("");
	}
}
