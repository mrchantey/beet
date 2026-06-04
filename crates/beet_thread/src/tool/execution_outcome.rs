use beet_core::prelude::*;

/// Parameters for the execution outcome tool.
/// Used by a model to confirm or deny that a process was successful.
#[derive(Debug, Clone, Reflect, serde::Serialize, serde::Deserialize)]
pub struct ExecutionOutcomeParams {
	/// Whether the execution passed or failed.
	pub passed: bool,
	/// Description of what went wrong, required when `passed` is false.
	#[serde(default)]
	pub reason: String,
}

/// A tool giving a model the opportunity to confirm success
/// or produce an error when a process behaved incorrectly.
#[action]
#[derive(Component, Reflect)]
pub async fn ExecutionOutcome(
	cx: ActionContext<ExecutionOutcomeParams>,
) -> Result<()> {
	if cx.input.passed {
		Ok(())
	} else {
		bevybail!("Execution failed: {}", cx.input.reason)
	}
}
