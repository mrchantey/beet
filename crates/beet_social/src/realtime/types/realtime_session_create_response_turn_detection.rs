use serde::Deserialize;
use serde::Serialize;

/// RealtimeSessionCreateResponseTurnDetection : Configuration for turn detection. Can be set to `null` to turn off. Server  VAD means that the model will detect the start and end of speech based on  audio volume and respond at the end of user speech.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RealtimeSessionCreateResponseTurnDetection {
	/// Type of turn detection, only `server_vad` is currently supported.
	#[serde(rename = "type", skip_serializing_if = "Option::is_none")]
	pub r#type: Option<String>,
	/// Activation threshold for VAD (0.0 to 1.0), this defaults to 0.5. A  higher threshold will require louder audio to activate the model, and  thus might perform better in noisy environments.
	#[serde(rename = "threshold", skip_serializing_if = "Option::is_none")]
	pub threshold: Option<f64>,
	/// Amount of audio to include before the VAD detected speech (in  milliseconds). Defaults to 300ms.
	#[serde(
		rename = "prefix_padding_ms",
		skip_serializing_if = "Option::is_none"
	)]
	pub prefix_padding_ms: Option<i32>,
	/// Duration of silence to detect speech stop (in milliseconds). Defaults  to 500ms. With shorter values the model will respond more quickly,  but may jump in on short pauses from the user.
	#[serde(
		rename = "silence_duration_ms",
		skip_serializing_if = "Option::is_none"
	)]
	pub silence_duration_ms: Option<i32>,
}

impl RealtimeSessionCreateResponseTurnDetection {
	/// Configuration for turn detection. Can be set to `null` to turn off. Server  VAD means that the model will detect the start and end of speech based on  audio volume and respond at the end of user speech.
	pub fn new() -> RealtimeSessionCreateResponseTurnDetection {
		RealtimeSessionCreateResponseTurnDetection {
			r#type: None,
			threshold: None,
			prefix_padding_ms: None,
			silence_duration_ms: None,
		}
	}
}
