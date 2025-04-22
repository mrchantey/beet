use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RealtimeConversationItemContentInner {
	/// The content type (`input_text`, `input_audio`, `item_reference`, `text`).
	#[serde(rename = "type", skip_serializing_if = "Option::is_none")]
	pub r#type: Option<Type>,
	/// The text content, used for `input_text` and `text` content types.
	#[serde(rename = "text", skip_serializing_if = "Option::is_none")]
	pub text: Option<String>,
	/// ID of a previous conversation item to reference (for `item_reference` content types in `response.create` events). These can reference both client and server created items.
	#[serde(rename = "id", skip_serializing_if = "Option::is_none")]
	pub id: Option<String>,
	/// Base64-encoded audio bytes, used for `input_audio` content type.
	#[serde(rename = "audio", skip_serializing_if = "Option::is_none")]
	pub audio: Option<String>,
	/// The transcript of the audio, used for `input_audio` content type.
	#[serde(rename = "transcript", skip_serializing_if = "Option::is_none")]
	pub transcript: Option<String>,
}

impl RealtimeConversationItemContentInner {
	pub fn new() -> RealtimeConversationItemContentInner {
		RealtimeConversationItemContentInner {
			r#type: None,
			text: None,
			id: None,
			audio: None,
			transcript: None,
		}
	}
}
/// The content type (`input_text`, `input_audio`, `item_reference`, `text`).
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
	#[serde(rename = "input_audio")]
	InputAudio,
	#[serde(rename = "input_text")]
	InputText,
	#[serde(rename = "item_reference")]
	ItemReference,
	#[serde(rename = "text")]
	Text,
}

impl Default for Type {
	fn default() -> Type { Self::InputAudio }
}
