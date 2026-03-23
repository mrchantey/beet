use serde::Deserialize;
use serde::Serialize;

/// RealtimeResponseCreateParamsConversation : Controls which conversation the response is added to. Currently supports `auto` and `none`, with `auto` as the default value. The `auto` value means that the contents of the response will be added to the default conversation. Set this to `none` to create an out-of-band response which  will not add items to default conversation.
/// Controls which conversation the response is added to. Currently supports `auto` and `none`, with `auto` as the default value. The `auto` value means that the contents of the response will be added to the default conversation. Set this to `none` to create an out-of-band response which  will not add items to default conversation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RealtimeResponseCreateParamsConversation {
	String(String),
}

impl Default for RealtimeResponseCreateParamsConversation {
	fn default() -> Self { Self::String(Default::default()) }
}
