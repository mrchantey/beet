//! Tool types for the OpenResponses API.
//!
//! This module contains types for defining and using tools (functions)
//! that the model can call during response generation.

use serde::Deserialize;
use serde::Serialize;

/// A function tool definition for use in requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionToolParam {
	/// The type of the tool. Always `"function"`.
	#[serde(rename = "type")]
	pub tool_type: String,
	/// The name of the function to call.
	pub name: String,
	/// A description of the function. Used by the model to decide when to call it.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	/// A JSON schema object describing the function parameters.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub parameters: Option<serde_json::Value>,
	/// Whether to enforce strict parameter validation. Defaults to `true`.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub strict: Option<bool>,
}

impl FunctionToolParam {
	/// Creates a new function tool with the given name.
	///
	/// # Example
	///
	/// ```no_run
	/// use beet_agent::prelude::openresponses::FunctionToolParam;
	///
	/// let tool = FunctionToolParam::new("get_weather")
	///     .with_description("Get the current weather for a location")
	///     .with_parameters(serde_json::json!({
	///         "type": "object",
	///         "properties": {
	///             "location": {
	///                 "type": "string",
	///                 "description": "The city and state, e.g. San Francisco, CA"
	///             }
	///         },
	///         "required": ["location"]
	///     }));
	///
	/// assert_eq!(tool.name, "get_weather");
	/// ```
	pub fn new(name: impl Into<String>) -> Self {
		Self {
			tool_type: "function".to_string(),
			name: name.into(),
			description: None,
			parameters: None,
			strict: None,
		}
	}

	/// Sets the function description.
	pub fn with_description(mut self, description: impl Into<String>) -> Self {
		self.description = Some(description.into());
		self
	}

	/// Sets the function parameters schema.
	pub fn with_parameters(mut self, parameters: serde_json::Value) -> Self {
		self.parameters = Some(parameters);
		self
	}

	/// Sets strict parameter validation.
	pub fn with_strict(mut self, strict: bool) -> Self {
		self.strict = Some(strict);
		self
	}
}

/// A function tool as returned in responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionTool {
	/// The type of the tool. Always `"function"`.
	#[serde(rename = "type")]
	pub tool_type: String,
	/// The name of the function.
	pub name: String,
	/// A description of the function.
	#[serde(default)]
	pub description: Option<String>,
	/// The parameters schema.
	#[serde(default)]
	pub parameters: Option<serde_json::Value>,
	/// Whether strict parameter validation is enforced.
	#[serde(default)]
	pub strict: Option<bool>,
}

/// Tool choice parameter - controls which tool the model should use.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
	/// A simple tool choice value.
	Value(super::ToolChoiceValue),
	/// A specific function to call.
	Function(SpecificFunctionChoice),
	/// An allowed tools specification.
	AllowedTools(AllowedToolsChoice),
}

impl ToolChoice {
	/// Restrict the model from calling any tools.
	pub fn none() -> Self { Self::Value(super::ToolChoiceValue::None) }

	/// Let the model choose from the provided tools.
	pub fn auto() -> Self { Self::Value(super::ToolChoiceValue::Auto) }

	/// Require the model to call a tool.
	pub fn required() -> Self { Self::Value(super::ToolChoiceValue::Required) }

	/// Require the model to call a specific function.
	pub fn function(name: impl Into<String>) -> Self {
		Self::Function(SpecificFunctionChoice {
			choice_type: "function".to_string(),
			name: name.into(),
		})
	}
}

/// A specific function choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecificFunctionChoice {
	/// The type. Always `"function"`.
	#[serde(rename = "type")]
	pub choice_type: String,
	/// The name of the function to call.
	pub name: String,
}

/// An allowed tools choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowedToolsChoice {
	/// The type. Always `"allowed_tools"`.
	#[serde(rename = "type")]
	pub choice_type: String,
	/// The list of allowed tools.
	pub tools: Vec<SpecificFunctionChoice>,
	/// How to select a tool from the allowed set.
	pub mode: super::ToolChoiceValue,
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn serializes_function_tool() {
		let tool = FunctionToolParam::new("get_weather")
			.with_description("Get weather info")
			.with_parameters(serde_json::json!({
				"type": "object",
				"properties": {
					"location": {"type": "string"}
				},
				"required": ["location"]
			}));

		let json = serde_json::to_value(&tool).unwrap();
		assert_eq!(json["type"], "function");
		assert_eq!(json["name"], "get_weather");
		assert_eq!(json["description"], "Get weather info");
		assert!(json["parameters"]["properties"]["location"].is_object());
	}

	#[test]
	fn serializes_tool_choice() {
		let choice = ToolChoice::auto();
		let json = serde_json::to_value(&choice).unwrap();
		assert_eq!(json, "auto");

		let choice = ToolChoice::function("get_weather");
		let json = serde_json::to_value(&choice).unwrap();
		assert_eq!(json["type"], "function");
		assert_eq!(json["name"], "get_weather");
	}
}
