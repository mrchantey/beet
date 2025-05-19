use crate::prelude::*;
use anyhow::Result;
pub struct LimboUtils;

impl LimboUtils {
	pub async fn collect_rows(mut rows_in: limbo::Rows) -> Result<Rows> {
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
