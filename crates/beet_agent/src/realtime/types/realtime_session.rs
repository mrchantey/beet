use crate::realtime::types as models;
use serde::Deserialize;
use serde::Serialize;

/// RealtimeSession : Realtime session object configuration.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RealtimeSession {
	/// Unique identifier for the session that looks like `sess_1234567890abcdef`.
	#[serde(rename = "id", skip_serializing_if = "Option::is_none")]
	pub id: Option<String>,
	/// The set of modalities the model can respond with. To disable audio, set this to [\"text\"].
	#[serde(rename = "modalities", skip_serializing_if = "Option::is_none")]
	pub modalities: Option<Vec<Modalities>>,
	/// The Realtime model used for this session.
	#[serde(rename = "model", skip_serializing_if = "Option::is_none")]
	pub model: Option<Model>,
	/// The default system instructions (i.e. system message) prepended to model  calls. This field allows the client to guide the model on desired  responses. The model can be instructed on response content and format,  (e.g. \"be extremely succinct\", \"act friendly\", \"here are examples of good  responses\") and on audio behavior (e.g. \"talk quickly\", \"inject emotion  into your voice\", \"laugh frequently\"). The instructions are not guaranteed  to be followed by the model, but they provide guidance to the model on the desired behavior.  Note that the server sets default instructions which will be used if this  field is not set and are visible in the `session.created` event at the  start of the session.
	#[serde(rename = "instructions", skip_serializing_if = "Option::is_none")]
	pub instructions: Option<String>,
	#[serde(rename = "voice", skip_serializing_if = "Option::is_none")]
	pub voice: Option<Box<models::VoiceIdsShared>>,
	/// The format of input audio. Options are `pcm16`, `g711_ulaw`, or `g711_alaw`. For `pcm16`, input audio must be 16-bit PCM at a 24kHz sample rate,  single channel (mono), and little-endian byte order.
	#[serde(
		rename = "input_audio_format",
		skip_serializing_if = "Option::is_none"
	)]
	pub input_audio_format: Option<InputAudioFormat>,
	/// The format of output audio. Options are `pcm16`, `g711_ulaw`, or `g711_alaw`. For `pcm16`, output audio is sampled at a rate of 24kHz.
	#[serde(
		rename = "output_audio_format",
		skip_serializing_if = "Option::is_none"
	)]
	pub output_audio_format: Option<OutputAudioFormat>,
	#[serde(
		rename = "input_audio_transcription",
		skip_serializing_if = "Option::is_none"
	)]
	pub input_audio_transcription:
		Option<Box<models::RealtimeSessionInputAudioTranscription>>,
	#[serde(
		rename = "turn_detection",
		skip_serializing_if = "Option::is_none"
	)]
	pub turn_detection: Option<Box<models::RealtimeSessionTurnDetection>>,
	#[serde(
		rename = "input_audio_noise_reduction",
		skip_serializing_if = "Option::is_none"
	)]
	pub input_audio_noise_reduction:
		Option<Box<models::RealtimeSessionInputAudioNoiseReduction>>,
	/// Tools (functions) available to the model.
	#[serde(rename = "tools", skip_serializing_if = "Option::is_none")]
	pub tools: Option<Vec<models::RealtimeResponseCreateParamsToolsInner>>,
	/// How the model chooses tools. Options are `auto`, `none`, `required`, or  specify a function.
	#[serde(rename = "tool_choice", skip_serializing_if = "Option::is_none")]
	pub tool_choice: Option<String>,
	/// Sampling temperature for the model, limited to [0.6, 1.2]. For audio models a temperature of 0.8 is highly recommended for best performance.
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

impl RealtimeSession {
	/// Realtime session object configuration.
	pub fn new() -> RealtimeSession {
		RealtimeSession {
			id: None,
			modalities: None,
			model: None,
			instructions: None,
			voice: None,
			input_audio_format: None,
			output_audio_format: None,
			input_audio_transcription: None,
			turn_detection: None,
			input_audio_noise_reduction: None,
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
/// The Realtime model used for this session.
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
pub enum Model {
	#[serde(rename = "gpt-4o-realtime-preview")]
	Gpt4oRealtimePreview,
	#[serde(rename = "gpt-4o-realtime-preview-2024-10-01")]
	Gpt4oRealtimePreview20241001,
	#[serde(rename = "gpt-4o-realtime-preview-2024-12-17")]
	Gpt4oRealtimePreview20241217,
	#[serde(rename = "gpt-4o-mini-realtime-preview")]
	Gpt4oMiniRealtimePreview,
	#[serde(rename = "gpt-4o-mini-realtime-preview-2024-12-17")]
	Gpt4oMiniRealtimePreview20241217,
}

impl Default for Model {
	fn default() -> Model { Self::Gpt4oRealtimePreview }
}
/// The format of input audio. Options are `pcm16`, `g711_ulaw`, or `g711_alaw`. For `pcm16`, input audio must be 16-bit PCM at a 24kHz sample rate,  single channel (mono), and little-endian byte order.
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
pub enum InputAudioFormat {
	#[serde(rename = "pcm16")]
	Pcm16,
	#[serde(rename = "g711_ulaw")]
	G711Ulaw,
	#[serde(rename = "g711_alaw")]
	G711Alaw,
}

impl Default for InputAudioFormat {
	fn default() -> InputAudioFormat { Self::Pcm16 }
}
/// The format of output audio. Options are `pcm16`, `g711_ulaw`, or `g711_alaw`. For `pcm16`, output audio is sampled at a rate of 24kHz.
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
