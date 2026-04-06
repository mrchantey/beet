use serde::Deserialize;
use serde::Serialize;

/// RealtimeSessionInputAudioNoiseReduction : Configuration for input audio noise reduction. This can be set to `null` to turn off. Noise reduction filters audio added to the input audio buffer before it is sent to VAD and the model. Filtering the audio can improve VAD and turn detection accuracy (reducing false positives) and model performance by improving perception of the input audio.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RealtimeSessionInputAudioNoiseReduction {
	/// Type of noise reduction. `near_field` is for close-talking microphones such as headphones, `far_field` is for far-field microphones such as laptop or conference room microphones.
	#[serde(rename = "type", skip_serializing_if = "Option::is_none")]
	pub r#type: Option<Type>,
}

impl RealtimeSessionInputAudioNoiseReduction {
	/// Configuration for input audio noise reduction. This can be set to `null` to turn off. Noise reduction filters audio added to the input audio buffer before it is sent to VAD and the model. Filtering the audio can improve VAD and turn detection accuracy (reducing false positives) and model performance by improving perception of the input audio.
	pub fn new() -> RealtimeSessionInputAudioNoiseReduction {
		RealtimeSessionInputAudioNoiseReduction { r#type: None }
	}
}
/// Type of noise reduction. `near_field` is for close-talking microphones such as headphones, `far_field` is for far-field microphones such as laptop or conference room microphones.
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
	#[serde(rename = "near_field")]
	NearField,
	#[serde(rename = "far_field")]
	FarField,
}

impl Default for Type {
	fn default() -> Type { Self::NearField }
}
