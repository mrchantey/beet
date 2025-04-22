use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RealtimeResponseCreateParamsToolsInner {
	/// The type of the tool, i.e. `function`.
	#[serde(rename = "type", skip_serializing_if = "Option::is_none")]
	pub r#type: Option<Type>,
	/// The name of the function.
	#[serde(rename = "name", skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,
	/// The description of the function, including guidance on when and how  to call it, and guidance about what to tell the user when calling  (if anything).
	#[serde(rename = "description", skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	/// Parameters of the function in JSON Schema.
	#[serde(rename = "parameters", skip_serializing_if = "Option::is_none")]
	pub parameters: Option<serde_json::Value>,
}

impl RealtimeResponseCreateParamsToolsInner {
	pub fn new() -> RealtimeResponseCreateParamsToolsInner {
		RealtimeResponseCreateParamsToolsInner {
			r#type: None,
			name: None,
			description: None,
			parameters: None,
		}
	}
}
/// The type of the tool, i.e. `function`.
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
	#[serde(rename = "function")]
	Function,
}

impl Default for Type {
	fn default() -> Type { Self::Function }
}
