/// A parsed limbo_core::vdbe::StepResult into an error
#[derive(Debug, thiserror::Error)]
pub enum StepError {
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




pub fn step_result_err(result: u64) -> Result<(), StepError> {
	match result {
		0 => Ok(()),
		1 => Err(StepError::IO),
		2 => Err(StepError::Row),
		3 => Err(StepError::Interrupt),
		4 => Err(StepError::Busy),
		code => Err(StepError::UnknownCode(code)),
	}
}
