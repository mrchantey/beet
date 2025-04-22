use serde::Deserialize;
use serde::Serialize;

/// The voices available for the Realtime API.
/// https://platform.openai.com/docs/api-reference/realtime-sessions/create#realtime-sessions-create-voice
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub enum VoiceIdsShared {
	#[serde(rename = "alloy")]
	Alloy,
	#[serde(rename = "ash")]
	Ash,
	#[serde(rename = "ballad")]
	Ballad,
	#[serde(rename = "coral")]
	Coral,
	#[serde(rename = "echo")]
	Echo,
	#[serde(rename = "fable")]
	Fable,
	#[serde(rename = "onyx")]
	Onyx,
	#[serde(rename = "nova")]
	Nova,
	#[serde(rename = "sage")]
	Sage,
	#[serde(rename = "shimmer")]
	Shimmer,
	#[serde(rename = "verse")]
	#[default]
	Verse,
}
