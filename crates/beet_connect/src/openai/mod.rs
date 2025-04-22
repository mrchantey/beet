use sweet::prelude::*;

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
	#[error("Failed to serialize request: {0}")]
	SerializationFailed(serde_json::Error),
	#[error("Request failed with status code: {0}")]
	RequestFailed(reqwest::StatusCode),
	#[error("Request failed with error: {0}")]
	RequestError(#[from] reqwest::Error),
	#[error("Failed to deserialize response: {0}")]
	DeserializationFailed(serde_json::Error),
	#[error("Invalid response from OpenAI API")]
	InvalidResponse,
}
