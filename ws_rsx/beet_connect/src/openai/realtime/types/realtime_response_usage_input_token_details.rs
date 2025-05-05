use serde::Deserialize;
use serde::Serialize;

/// RealtimeResponseUsageInputTokenDetails : Details about the input tokens used in the Response.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RealtimeResponseUsageInputTokenDetails {
	/// The number of cached tokens used in the Response.
	#[serde(rename = "cached_tokens", skip_serializing_if = "Option::is_none")]
	pub cached_tokens: Option<i32>,
	/// The number of text tokens used in the Response.
	#[serde(rename = "text_tokens", skip_serializing_if = "Option::is_none")]
	pub text_tokens: Option<i32>,
	/// The number of audio tokens used in the Response.
	#[serde(rename = "audio_tokens", skip_serializing_if = "Option::is_none")]
	pub audio_tokens: Option<i32>,
}

impl RealtimeResponseUsageInputTokenDetails {
	/// Details about the input tokens used in the Response.
	pub fn new() -> RealtimeResponseUsageInputTokenDetails {
		RealtimeResponseUsageInputTokenDetails {
			cached_tokens: None,
			text_tokens: None,
			audio_tokens: None,
		}
	}
}
