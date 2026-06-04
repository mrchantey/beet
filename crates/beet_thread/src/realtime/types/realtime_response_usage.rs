use crate::realtime::types as models;
use serde::Deserialize;
use serde::Serialize;

/// RealtimeResponseUsage : Usage statistics for the Response, this will correspond to billing. A  Realtime API session will maintain a conversation context and append new  Items to the Conversation, thus output from previous turns (text and  audio tokens) will become the input for later turns.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RealtimeResponseUsage {
	/// The total number of tokens in the Response including input and output  text and audio tokens.
	#[serde(rename = "total_tokens", skip_serializing_if = "Option::is_none")]
	pub total_tokens: Option<i32>,
	/// The number of input tokens used in the Response, including text and  audio tokens.
	#[serde(rename = "input_tokens", skip_serializing_if = "Option::is_none")]
	pub input_tokens: Option<i32>,
	/// The number of output tokens sent in the Response, including text and  audio tokens.
	#[serde(rename = "output_tokens", skip_serializing_if = "Option::is_none")]
	pub output_tokens: Option<i32>,
	#[serde(
		rename = "input_token_details",
		skip_serializing_if = "Option::is_none"
	)]
	pub input_token_details:
		Option<Box<models::RealtimeResponseUsageInputTokenDetails>>,
	#[serde(
		rename = "output_token_details",
		skip_serializing_if = "Option::is_none"
	)]
	pub output_token_details:
		Option<Box<models::RealtimeResponseUsageOutputTokenDetails>>,
}

impl RealtimeResponseUsage {
	/// Usage statistics for the Response, this will correspond to billing. A  Realtime API session will maintain a conversation context and append new  Items to the Conversation, thus output from previous turns (text and  audio tokens) will become the input for later turns.
	pub fn new() -> RealtimeResponseUsage {
		RealtimeResponseUsage {
			total_tokens: None,
			input_tokens: None,
			output_tokens: None,
			input_token_details: None,
			output_token_details: None,
		}
	}
}
