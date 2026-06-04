use serde::Deserialize;
use serde::Serialize;

/// RealtimeSessionInputAudioTranscription : Configuration for input audio transcription, defaults to off and can be  set to `null` to turn off once on. Input audio transcription is not native to the model, since the model consumes audio directly. Transcription runs  asynchronously through [the /audio/transcriptions endpoint](https://platform.openai.com/docs/api-reference/audio/createTranscription) and should be treated as guidance of input audio content rather than precisely what the model heard. The client can optionally set the language and prompt for transcription, these offer additional guidance to the transcription service.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RealtimeSessionInputAudioTranscription {
	/// The model to use for transcription, current options are `gpt-4o-transcribe`, `gpt-4o-mini-transcribe`, and `whisper-1`.
	#[serde(rename = "model", skip_serializing_if = "Option::is_none")]
	pub model: Option<String>,
	/// The language of the input audio. Supplying the input language in [ISO-639-1](https://en.wikipedia.org/wiki/List_of_ISO_639-1_codes) (e.g. `en`) format will improve accuracy and latency.
	#[serde(rename = "language", skip_serializing_if = "Option::is_none")]
	pub language: Option<String>,
	/// An optional text to guide the model's style or continue a previous audio segment. For `whisper-1`, the [prompt is a list of keywords](/docs/guides/speech-to-text#prompting). For `gpt-4o-transcribe` models, the prompt is a free text string, for example \"expect words related to technology\".
	#[serde(rename = "prompt", skip_serializing_if = "Option::is_none")]
	pub prompt: Option<String>,
}

impl RealtimeSessionInputAudioTranscription {
	/// Configuration for input audio transcription, defaults to off and can be  set to `null` to turn off once on. Input audio transcription is not native to the model, since the model consumes audio directly. Transcription runs  asynchronously through [the /audio/transcriptions endpoint](https://platform.openai.com/docs/api-reference/audio/createTranscription) and should be treated as guidance of input audio content rather than precisely what the model heard. The client can optionally set the language and prompt for transcription, these offer additional guidance to the transcription service.
	pub fn new() -> RealtimeSessionInputAudioTranscription {
		RealtimeSessionInputAudioTranscription {
			model: None,
			language: None,
			prompt: None,
		}
	}
}
