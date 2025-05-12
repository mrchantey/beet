use crate::openai::realtime::types as models;
use serde::Deserialize;
use serde::Serialize;

/// RealtimeResponseStatusDetails : Additional details about the status.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RealtimeResponseStatusDetails {
	/// The type of error that caused the response to fail, corresponding  with the `status` field (`completed`, `cancelled`, `incomplete`,  `failed`).
	#[serde(rename = "type", skip_serializing_if = "Option::is_none")]
	pub r#type: Option<Type>,
	/// The reason the Response did not complete. For a `cancelled` Response,  one of `turn_detected` (the server VAD detected a new start of speech)  or `client_cancelled` (the client sent a cancel event). For an  `incomplete` Response, one of `max_output_tokens` or `content_filter`  (the server-side safety filter activated and cut off the response).
	#[serde(rename = "reason", skip_serializing_if = "Option::is_none")]
	pub reason: Option<Reason>,
	#[serde(rename = "error", skip_serializing_if = "Option::is_none")]
	pub error: Option<Box<models::RealtimeResponseStatusDetailsError>>,
}

impl RealtimeResponseStatusDetails {
	/// Additional details about the status.
	pub fn new() -> RealtimeResponseStatusDetails {
		RealtimeResponseStatusDetails {
			r#type: None,
			reason: None,
			error: None,
		}
	}
}
/// The type of error that caused the response to fail, corresponding  with the `status` field (`completed`, `cancelled`, `incomplete`,  `failed`).
#[derive(
	Clone,
	Copy,
	Debug,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Hash,
	Serialize,
	Deserialize,
)]
pub enum Type {
	#[serde(rename = "completed")]
	Completed,
	#[serde(rename = "cancelled")]
	Cancelled,
	#[serde(rename = "failed")]
	Failed,
	#[serde(rename = "incomplete")]
	Incomplete,
}

impl Default for Type {
	fn default() -> Type { Self::Completed }
}
/// The reason the Response did not complete. For a `cancelled` Response,  one of `turn_detected` (the server VAD detected a new start of speech)  or `client_cancelled` (the client sent a cancel event). For an  `incomplete` Response, one of `max_output_tokens` or `content_filter`  (the server-side safety filter activated and cut off the response).
#[derive(
	Clone,
	Copy,
	Debug,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Hash,
	Serialize,
	Deserialize,
)]
pub enum Reason {
	#[serde(rename = "turn_detected")]
	TurnDetected,
	#[serde(rename = "client_cancelled")]
	ClientCancelled,
	#[serde(rename = "max_output_tokens")]
	MaxOutputTokens,
	#[serde(rename = "content_filter")]
	ContentFilter,
}

impl Default for Reason {
	fn default() -> Reason { Self::TurnDetected }
}
