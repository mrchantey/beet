use beet_core::net::cross_fetch;
use beet_utils::prelude::*;

pub mod realtime;





pub struct OpenAiKey;

impl OpenAiKey {
	/// Load the `OPENAI_API_KEY` from the environment variables.
	pub fn get() -> OpenAiResult<String> {
		std::env::var("OPENAI_API_KEY")?.xok()
	}
}




pub type OpenAiResult<T> = Result<T, OpenAiError>;

#[derive(Debug, thiserror::Error)]
pub enum OpenAiError {
	#[error("No API key found")]
	NoApiKey(#[from] std::env::VarError),
	#[error("{0}")]
	FetchError(#[from] cross_fetch::Error),
	#[error("Invalid response from OpenAI API")]
	InvalidResponse,
}
