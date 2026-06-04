use serde::Deserialize;
use serde::Serialize;

/// RealtimeSessionTurnDetection : Configuration for turn detection, ether Server VAD or Semantic VAD. This can be set to `null` to turn off, in which case the client must manually trigger model response. Server VAD means that the model will detect the start and end of speech based on audio volume and respond at the end of user speech. Semantic VAD is more advanced and uses a turn detection model (in conjuction with VAD) to semantically estimate whether the user has finished speaking, then dynamically sets a timeout based on this probability. For example, if user audio trails off with \"uhhm\", the model will score a low probability of turn end and wait longer for the user to continue speaking. This can be useful for more natural conversations, but may have a higher latency.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RealtimeSessionTurnDetection {
	/// Type of turn detection.
	#[serde(rename = "type", skip_serializing_if = "Option::is_none")]
	pub r#type: Option<Type>,
	/// Used only for `semantic_vad` mode. The eagerness of the model to respond. `low` will wait longer for the user to continue speaking, `high` will respond more quickly. `auto` is the default and is equivalent to `medium`.
	#[serde(rename = "eagerness", skip_serializing_if = "Option::is_none")]
	pub eagerness: Option<Eagerness>,
	/// Used only for `server_vad` mode. Activation threshold for VAD (0.0 to 1.0), this defaults to 0.5. A  higher threshold will require louder audio to activate the model, and  thus might perform better in noisy environments.
	#[serde(rename = "threshold", skip_serializing_if = "Option::is_none")]
	pub threshold: Option<f64>,
	/// Used only for `server_vad` mode. Amount of audio to include before the VAD detected speech (in  milliseconds). Defaults to 300ms.
	#[serde(
		rename = "prefix_padding_ms",
		skip_serializing_if = "Option::is_none"
	)]
	pub prefix_padding_ms: Option<i32>,
	/// Used only for `server_vad` mode. Duration of silence to detect speech stop (in milliseconds). Defaults  to 500ms. With shorter values the model will respond more quickly,  but may jump in on short pauses from the user.
	#[serde(
		rename = "silence_duration_ms",
		skip_serializing_if = "Option::is_none"
	)]
	pub silence_duration_ms: Option<i32>,
	/// Whether or not to automatically generate a response when a VAD stop event occurs.
	#[serde(
		rename = "create_response",
		skip_serializing_if = "Option::is_none"
	)]
	pub create_response: Option<bool>,
	/// Whether or not to automatically interrupt any ongoing response with output to the default conversation (i.e. `conversation` of `auto`) when a VAD start event occurs.
	#[serde(
		rename = "interrupt_response",
		skip_serializing_if = "Option::is_none"
	)]
	pub interrupt_response: Option<bool>,
}

impl RealtimeSessionTurnDetection {
	/// Configuration for turn detection, ether Server VAD or Semantic VAD. This can be set to `null` to turn off, in which case the client must manually trigger model response. Server VAD means that the model will detect the start and end of speech based on audio volume and respond at the end of user speech. Semantic VAD is more advanced and uses a turn detection model (in conjuction with VAD) to semantically estimate whether the user has finished speaking, then dynamically sets a timeout based on this probability. For example, if user audio trails off with \"uhhm\", the model will score a low probability of turn end and wait longer for the user to continue speaking. This can be useful for more natural conversations, but may have a higher latency.
	pub fn new() -> RealtimeSessionTurnDetection {
		RealtimeSessionTurnDetection {
			r#type: None,
			eagerness: None,
			threshold: None,
			prefix_padding_ms: None,
			silence_duration_ms: None,
			create_response: None,
			interrupt_response: None,
		}
	}
}
/// Type of turn detection.
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
	#[serde(rename = "server_vad")]
	ServerVad,
	#[serde(rename = "semantic_vad")]
	SemanticVad,
}

impl Default for Type {
	fn default() -> Type { Self::ServerVad }
}
/// Used only for `semantic_vad` mode. The eagerness of the model to respond. `low` will wait longer for the user to continue speaking, `high` will respond more quickly. `auto` is the default and is equivalent to `medium`.
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
pub enum Eagerness {
	#[serde(rename = "low")]
	Low,
	#[serde(rename = "medium")]
	Medium,
	#[serde(rename = "high")]
	High,
	#[serde(rename = "auto")]
	Auto,
}

impl Default for Eagerness {
	fn default() -> Eagerness { Self::Low }
}
