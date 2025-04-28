use crate::prelude::*;
use anyhow::Result;
use libsql::*;
use sweet::prelude::*;

pub struct LibsqlUtils;

impl LibsqlUtils {
	pub async fn memory_db() -> Result<libsql::Connection, libsql::Error> {
		Builder::new_local(":memory:").build().await?.connect()
	}
	pub async fn remote_db() -> Result<libsql::Connection> {
		let url = std::env::var("LIBSQL_URL")?;
		let token = std::env::var("LIBSQL_AUTH_TOKEN").unwrap_or_else(|_| {
			println!("LIBSQL_TOKEN not set, using empty token...");
			"".to_string()
		});
		Builder::new_remote(url, token)
			.build()
			.await?
			.connect()?
			.xok()
	}

	pub fn parse_step_result(result: usize) -> LibsqlStepResult {
		match result {
			0 => LibsqlStepResult::Done,
			1 => LibsqlStepResult::IO,
			2 => LibsqlStepResult::Row,
			3 => LibsqlStepResult::Interrupt,
			4 => LibsqlStepResult::Busy,
			code => LibsqlStepResult::UnknownCode(code),
		}
	}

	pub async fn collect_rows(mut rows: libsql::Rows) -> Result<SeaQueryRows> {
		let mut result = Vec::new();
		while let Some(row) = rows.next().await? {
			let mut cells = Vec::new();
			for i in 0..row.column_count() {
				let value = row.get_value(i)?;
				cells.push(value.into_sea_query_value());
			}
			result.push(cells);
		}
		Ok(result)
	}
}

/// A parsed libsql_core::vdbe::StepResult into an error
#[derive(Debug)]
pub enum LibsqlStepResult {
	Done,
	IO,
	Row,
	Interrupt,
	Busy,
	UnknownCode(usize),
}
