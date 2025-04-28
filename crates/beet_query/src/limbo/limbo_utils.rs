use crate::prelude::*;
use limbo::*;

pub struct LimboUtils;

impl LimboUtils {
	pub async fn memory_db() -> Result<limbo::Connection> {
		Builder::new_local(":memory:").build().await?.connect()
	}


	pub async fn collect_rows(mut rows: limbo::Rows) -> Result<SeaQueryRows> {
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

	pub fn step_result_err(
		result: u64,
	) -> std::result::Result<(), LimboStepError> {
		match result {
			0 => Ok(()),
			1 => Err(LimboStepError::IO),
			2 => Err(LimboStepError::Row),
			3 => Err(LimboStepError::Interrupt),
			4 => Err(LimboStepError::Busy),
			code => Err(LimboStepError::UnknownCode(code)),
		}
	}
}


/// A parsed limbo_core::vdbe::StepResult into an error
#[derive(Debug, thiserror::Error)]
pub enum LimboStepError {
	#[error("StepError::IO")]
	IO,
	#[error("StepError::Row")]
	Row,
	#[error("StepError::Interrupt")]
	Interrupt,
	#[error("StepError::Busy")]
	Busy,
	#[error("StepError::UnknownCode: {0}")]
	UnknownCode(u64),
}
