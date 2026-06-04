use serde::Deserialize;
use serde::Serialize;

/// RealtimeResponseCreateParamsMaxResponseOutputTokens : Maximum number of output tokens for a single assistant response, inclusive of tool calls. Provide an integer between 1 and 4096 to limit output tokens, or `inf` for the maximum available tokens for a given model. Defaults to `inf`.
/// Maximum number of output tokens for a single assistant response, inclusive of tool calls. Provide an integer between 1 and 4096 to limit output tokens, or `inf` for the maximum available tokens for a given model. Defaults to `inf`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RealtimeResponseCreateParamsMaxResponseOutputTokens {
	Integer(i32),
	String(String),
}

impl Default for RealtimeResponseCreateParamsMaxResponseOutputTokens {
	fn default() -> Self { Self::Integer(Default::default()) }
}
