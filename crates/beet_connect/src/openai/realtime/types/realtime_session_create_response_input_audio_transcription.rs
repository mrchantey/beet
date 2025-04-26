use serde::Deserialize;
use serde::Serialize;

/// RealtimeSessionCreateResponseInputAudioTranscription : Configuration for input audio transcription, defaults to off and can be  set to `null` to turn off once on. Input audio transcription is not native  to the model, since the model consumes audio directly. Transcription runs  asynchronously through Whisper and should be treated as rough guidance  rather than the representation understood by the model.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RealtimeSessionCreateResponseInputAudioTranscription {
	/// The model to use for transcription, `whisper-1` is the only currently  supported model.
	#[serde(rename = "model", skip_serializing_if = "Option::is_none")]
	pub model: Option<String>,
}

impl RealtimeSessionCreateResponseInputAudioTranscription {
	/// Configuration for input audio transcription, defaults to off and can be  set to `null` to turn off once on. Input audio transcription is not native  to the model, since the model consumes audio directly. Transcription runs  asynchronously through Whisper and should be treated as rough guidance  rather than the representation understood by the model.
	pub fn new() -> RealtimeSessionCreateResponseInputAudioTranscription {
		RealtimeSessionCreateResponseInputAudioTranscription { model: None }
	}
}
