use serde::Deserialize;
use serde::Serialize;

/// RealtimeResponseMaxOutputTokens : Maximum number of output tokens for a single assistant response, inclusive of tool calls, that was used in this response.
/// Maximum number of output tokens for a single assistant response, inclusive of tool calls, that was used in this response.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RealtimeResponseMaxOutputTokens {
	Integer(i32),
	String(String),
}

impl Default for RealtimeResponseMaxOutputTokens {
	fn default() -> Self { Self::Integer(Default::default()) }
}
