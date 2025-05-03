use serde::Deserialize;
use serde::Serialize;

/// RealtimeResponseUsageOutputTokenDetails : Details about the output tokens used in the Response.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RealtimeResponseUsageOutputTokenDetails {
	/// The number of text tokens used in the Response.
	#[serde(rename = "text_tokens", skip_serializing_if = "Option::is_none")]
	pub text_tokens: Option<i32>,
	/// The number of audio tokens used in the Response.
	#[serde(rename = "audio_tokens", skip_serializing_if = "Option::is_none")]
	pub audio_tokens: Option<i32>,
}

impl RealtimeResponseUsageOutputTokenDetails {
	/// Details about the output tokens used in the Response.
	pub fn new() -> RealtimeResponseUsageOutputTokenDetails {
		RealtimeResponseUsageOutputTokenDetails {
			text_tokens: None,
			audio_tokens: None,
		}
	}
}
