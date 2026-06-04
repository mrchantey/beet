use serde::Deserialize;
use serde::Serialize;

/// RealtimeResponseStatusDetailsError : A description of the error that caused the response to fail,  populated when the `status` is `failed`.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RealtimeResponseStatusDetailsError {
	/// The type of error.
	#[serde(rename = "type", skip_serializing_if = "Option::is_none")]
	pub r#type: Option<String>,
	/// Error code, if any.
	#[serde(rename = "code", skip_serializing_if = "Option::is_none")]
	pub code: Option<String>,
}

impl RealtimeResponseStatusDetailsError {
	/// A description of the error that caused the response to fail,  populated when the `status` is `failed`.
	pub fn new() -> RealtimeResponseStatusDetailsError {
		RealtimeResponseStatusDetailsError {
			r#type: None,
			code: None,
		}
	}
}
