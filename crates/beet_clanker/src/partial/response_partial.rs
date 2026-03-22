use crate::prelude::*;
use beet_core::prelude::*;

/// Represents a section or a whole of a response,
/// unifying both streaming and non-streaming transports
/// for model responses.
#[derive(Debug, Clone, PartialEq)]
pub struct ResponsePartial {
	pub response_id: String,
	/// whether the response was stored, enabling usage of
	/// `previous_response_id` in the next request.
	pub response_stored: bool,
	pub status: ResponseStatus,
	pub token_usage: Option<TokenUsage>,
	/// list of actions in this partial
	pub actions: Vec<ActionPartial>,
}

impl ResponsePartial {
	pub fn is_final(&self) -> bool {
		matches!(
			self.status,
			ResponseStatus::Completed
				| ResponseStatus::Failed { .. }
				| ResponseStatus::Cancelled
		)
	}
	pub fn take_actions(&mut self) -> Vec<ActionPartial> {
		std::mem::take(&mut self.actions)
	}
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResponseStatus {
	Created,
	Queued,
	InProgress,
	Completed,
	Failed {
		code: Option<String>,
		message: Option<String>,
	},
	Incomplete(Option<String>),
	Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TokenUsage {
	/// The number of input tokens used to generate the response.
	pub input_tokens: u32,
	/// The number of input tokens used to generate the response.
	pub output_tokens: u32,
	/// The total number of tokens used.
	pub total_tokens: u32,
	/// The number of input tokens that were served from cache.
	pub cached_input_tokens: Option<u32>,
	/// The number of output tokens attributed to reasoning.
	pub reasoning_tokens: Option<u32>,
}
