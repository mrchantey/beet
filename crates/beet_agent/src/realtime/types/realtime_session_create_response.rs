use crate::realtime::types as models;
use serde::Deserialize;
use serde::Serialize;

/// RealtimeSessionCreateResponse : A new Realtime session configuration, with an ephermeral key. Default TTL for keys is one minute.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RealtimeSessionCreateResponse {
	#[serde(rename = "client_secret")]
	pub client_secret: Box<models::RealtimeSessionCreateResponseClientSecret>,
	/// The set of modalities the model can respond with. To disable audio, set this to [\"text\"].
	#[serde(rename = "modalities", skip_serializing_if = "Option::is_none")]
	pub modalities: Option<Vec<Modalities>>,
	/// The default system instructions (i.e. system message) prepended to model  calls. This field allows the client to guide the model on desired  responses. The model can be instructed on response content and format,  (e.g. \"be extremely succinct\", \"act friendly\", \"here are examples of good  responses\") and on audio behavior (e.g. \"talk quickly\", \"inject emotion  into your voice\", \"laugh frequently\"). The instructions are not guaranteed  to be followed by the model, but they provide guidance to the model on the  desired behavior.  Note that the server sets default instructions which will be used if this  field is not set and are visible in the `session.created` event at the  start of the session.
	#[serde(rename = "instructions", skip_serializing_if = "Option::is_none")]
	pub instructions: Option<String>,
	#[serde(rename = "voice", skip_serializing_if = "Option::is_none")]
	pub voice: Option<Box<models::VoiceIdsShared>>,
	/// The format of input audio. Options are `pcm16`, `g711_ulaw`, or `g711_alaw`.
	#[serde(
		rename = "input_audio_format",
		skip_serializing_if = "Option::is_none"
	)]
	pub input_audio_format: Option<String>,
	/// The format of output audio. Options are `pcm16`, `g711_ulaw`, or `g711_alaw`.
	#[serde(
		rename = "output_audio_format",
		skip_serializing_if = "Option::is_none"
	)]
	pub output_audio_format: Option<String>,
	#[serde(
		rename = "input_audio_transcription",
		skip_serializing_if = "Option::is_none"
	)]
	pub input_audio_transcription: Option<
		Box<models::RealtimeSessionCreateResponseInputAudioTranscription>,
	>,
	#[serde(
		rename = "turn_detection",
		skip_serializing_if = "Option::is_none"
	)]
	pub turn_detection:
		Option<Box<models::RealtimeSessionCreateResponseTurnDetection>>,
	/// Tools (functions) available to the model.
	#[serde(rename = "tools", skip_serializing_if = "Option::is_none")]
	pub tools: Option<Vec<models::RealtimeResponseCreateParamsToolsInner>>,
	/// How the model chooses tools. Options are `auto`, `none`, `required`, or  specify a function.
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
}

impl RealtimeSessionCreateResponse {
	/// A new Realtime session configuration, with an ephermeral key. Default TTL for keys is one minute.
	pub fn new(
		client_secret: models::RealtimeSessionCreateResponseClientSecret,
	) -> RealtimeSessionCreateResponse {
		RealtimeSessionCreateResponse {
			client_secret: Box::new(client_secret),
			modalities: None,
			instructions: None,
			voice: None,
			input_audio_format: None,
			output_audio_format: None,
			input_audio_transcription: None,
			turn_detection: None,
			tools: None,
			tool_choice: None,
			temperature: None,
			max_response_output_tokens: None,
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
