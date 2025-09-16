use crate::realtime::types as models;
use serde::Deserialize;
use serde::Serialize;

/// RealtimeResponseCreateParams : Create a new Realtime response with these parameters
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RealtimeResponseCreateParams {
	/// The set of modalities the model can respond with. To disable audio, set this to [\"text\"].
	#[serde(rename = "modalities", skip_serializing_if = "Option::is_none")]
	pub modalities: Option<Vec<Modalities>>,
	/// The default system instructions (i.e. system message) prepended to model  calls. This field allows the client to guide the model on desired  responses. The model can be instructed on response content and format,  (e.g. \"be extremely succinct\", \"act friendly\", \"here are examples of good  responses\") and on audio behavior (e.g. \"talk quickly\", \"inject emotion  into your voice\", \"laugh frequently\"). The instructions are not guaranteed  to be followed by the model, but they provide guidance to the model on the  desired behavior.  Note that the server sets default instructions which will be used if this  field is not set and are visible in the `session.created` event at the  start of the session.
	#[serde(rename = "instructions", skip_serializing_if = "Option::is_none")]
	pub instructions: Option<String>,
	#[serde(rename = "voice", skip_serializing_if = "Option::is_none")]
	pub voice: Option<Box<models::VoiceIdsShared>>,
	/// The format of output audio. Options are `pcm16`, `g711_ulaw`, or `g711_alaw`.
	#[serde(
		rename = "output_audio_format",
		skip_serializing_if = "Option::is_none"
	)]
	pub output_audio_format: Option<OutputAudioFormat>,
	/// Tools (functions) available to the model.
	#[serde(rename = "tools", skip_serializing_if = "Option::is_none")]
	pub tools: Option<Vec<models::RealtimeResponseCreateParamsToolsInner>>,
	/// How the model chooses tools. Options are `auto`, `none`, `required`, or  specify a function, like `{\"type\": \"function\", \"function\": {\"name\": \"my_function\"}}`.
	#[serde(rename = "tool_choice", skip_serializing_if = "Option::is_none")]
	pub tool_choice: Option<String>,
	/// Sampling temperature for the model, limited to [0.6, 1.2]. Defaults to 0.8.
	#[serde(rename = "temperature", skip_serializing_if = "Option::is_none")]
	pub temperature: Option<f64>,
	#[serde(
		rename = "max_response_output_tokens",
		skip_serializing_if = "Option::is_none"
	)]
	pub max_response_output_tokens: Option<
		Box<models::RealtimeResponseCreateParamsMaxResponseOutputTokens>,
	>,
	#[serde(rename = "conversation", skip_serializing_if = "Option::is_none")]
	pub conversation:
		Option<Box<models::RealtimeResponseCreateParamsConversation>>,
	/// Set of 16 key-value pairs that can be attached to an object. This can be useful for storing additional information about the object in a structured format, and querying for objects via API or the dashboard.   Keys are strings with a maximum length of 64 characters. Values are strings with a maximum length of 512 characters.
	#[serde(
		rename = "metadata",
		default,
		with = "::serde_with::rust::double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub metadata: Option<Option<std::collections::HashMap<String, String>>>,
	/// Input items to include in the prompt for the model. Using this field creates a new context for this Response instead of using the default conversation. An empty array `[]` will clear the context for this Response. Note that this can include references to items from the default conversation.
	#[serde(rename = "input", skip_serializing_if = "Option::is_none")]
	pub input: Option<Vec<models::RealtimeConversationItemWithReference>>,
}

impl RealtimeResponseCreateParams {
	/// Create a new Realtime response with these parameters
	pub fn new() -> RealtimeResponseCreateParams {
		RealtimeResponseCreateParams {
			modalities: None,
			instructions: None,
			voice: None,
			output_audio_format: None,
			tools: None,
			tool_choice: None,
			temperature: None,
			max_response_output_tokens: None,
			conversation: None,
			metadata: None,
			input: None,
		}
	}
}
/// The set of modalities the model can respond with. To disable audio, set this to [\"text\"].
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
pub enum Modalities {
	#[serde(rename = "text")]
	Text,
	#[serde(rename = "audio")]
	Audio,
}

impl Default for Modalities {
	fn default() -> Modalities { Self::Text }
}
/// The format of output audio. Options are `pcm16`, `g711_ulaw`, or `g711_alaw`.
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
pub enum OutputAudioFormat {
	#[serde(rename = "pcm16")]
	Pcm16,
	#[serde(rename = "g711_ulaw")]
	G711Ulaw,
	#[serde(rename = "g711_alaw")]
	G711Alaw,
}

impl Default for OutputAudioFormat {
	fn default() -> OutputAudioFormat { Self::Pcm16 }
}
