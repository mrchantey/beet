use crate::prelude::*;
use anyhow::Result;

pub struct LibsqlUtils;

impl LibsqlUtils {


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

	pub async fn collect_rows(mut rows_in: libsql::Rows) -> Result<Rows> {
		let mut rows_out = Rows::default();
		while let Some(row_in) = rows_in.next().await? {
			let mut row_out = Row::default();
			for i in 0..row_in.column_count() {
				let value = row_in.get_value(i)?;
				row_out.push(value.into_value()?);
			}
			rows_out.push(row_out);
		}
		Ok(rows_out)
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
