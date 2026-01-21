//! Request types for the OpenResponses API.
//!
//! This module contains the request body and all related types for making
//! requests to the `/v1/responses` endpoint.
//!
//! # Example
//!
//! ```no_run
//! use beet_agent::prelude::openresponses;
//!
//! let body = openresponses::RequestBody::new("gpt-4o-mini")
//!     .with_input("Hello, world!")
//!     .with_temperature(0.7);
//!
//! assert_eq!(body.model, "gpt-4o-mini");
//! ```

use super::*;
use serde::Deserialize;
use serde::Serialize;

/// The request body for the `/v1/responses` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBody {
	/// The model to use for this request, e.g. `"gpt-4o-mini"`.
	pub model: String,
	/// Context to provide to the model. May be a string (interpreted as a user message)
	/// or an array of input items.
	pub input: Input,
	/// The ID of the response to use as the prior turn for this request.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub previous_response_id: Option<String>,
	/// Specifies additional output data to include in the response.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub include: Option<Vec<IncludeOption>>,
	/// A list of tools that the model may call while generating the response.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tools: Option<Vec<FunctionToolParam>>,
	/// Controls which tool the model should use, if any.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tool_choice: Option<ToolChoice>,
	/// Set of key-value pairs for storing additional information.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub metadata: Option<Metadata>,
	/// Configuration options for text output.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub text: Option<TextParam>,
	/// Sampling temperature between 0 and 2. Higher values make output more random.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub temperature: Option<f64>,
	/// Nucleus sampling parameter between 0 and 1.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub top_p: Option<f64>,
	/// Penalizes new tokens based on whether they appear in the text so far.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub presence_penalty: Option<f64>,
	/// Penalizes new tokens based on their frequency in the text so far.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub frequency_penalty: Option<f64>,
	/// Whether the model may call multiple tools in parallel.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub parallel_tool_calls: Option<bool>,
	/// Whether to stream response events as server-sent events.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub stream: Option<bool>,
	/// Options that control streamed response behavior.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub stream_options: Option<StreamOptions>,
	/// Whether to run the request in the background and return immediately.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub background: Option<bool>,
	/// Maximum number of tokens the model may generate.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub max_output_tokens: Option<u32>,
	/// Maximum number of tool calls the model may make.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub max_tool_calls: Option<u32>,
	/// Configuration options for reasoning behavior.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub reasoning: Option<ReasoningParam>,
	/// A stable identifier used for safety monitoring and abuse detection.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub safety_identifier: Option<String>,
	/// A key to use when reading from or writing to the prompt cache.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub prompt_cache_key: Option<String>,
	/// Controls how the service truncates input when it exceeds the model context window.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub truncation: Option<Truncation>,
	/// Additional instructions to guide the model for this request.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub instructions: Option<String>,
	/// Whether to store the response so it can be retrieved later.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub store: Option<bool>,
	/// The service tier to use for this request.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub service_tier: Option<ServiceTier>,
	/// Number of most likely tokens to return at each position with log probabilities.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub top_logprobs: Option<u32>,
}

impl RequestBody {
	/// Creates a new request body with the specified model.
	pub fn new(model: impl Into<String>) -> Self {
		Self {
			model: model.into(),
			input: Input::Items(Vec::new()),
			previous_response_id: None,
			include: None,
			tools: None,
			tool_choice: None,
			metadata: None,
			text: None,
			temperature: None,
			top_p: None,
			presence_penalty: None,
			frequency_penalty: None,
			parallel_tool_calls: None,
			stream: None,
			stream_options: None,
			background: None,
			max_output_tokens: None,
			max_tool_calls: None,
			reasoning: None,
			safety_identifier: None,
			prompt_cache_key: None,
			truncation: None,
			instructions: None,
			store: None,
			service_tier: None,
			top_logprobs: None,
		}
	}

	/// Sets the input as a simple string (interpreted as a user message).
	pub fn with_input(mut self, input: impl Into<String>) -> Self {
		self.input = Input::Text(input.into());
		self
	}

	/// Sets the input as an array of input items.
	pub fn with_input_items(mut self, items: Vec<InputItem>) -> Self {
		self.input = Input::Items(items);
		self
	}

	/// Adds a single input item to the request.
	pub fn with_input_item(mut self, item: InputItem) -> Self {
		match &mut self.input {
			Input::Items(items) => items.push(item),
			Input::Text(text) => {
				let user_msg = InputItem::Message(MessageParam {
					id: None,
					role: MessageRole::User,
					content: MessageContent::Text(std::mem::take(text)),
					status: None,
				});
				self.input = Input::Items(vec![user_msg, item]);
			}
		}
		self
	}

	/// Sets the previous response ID for multi-turn conversations.
	pub fn with_previous_response_id(mut self, id: impl Into<String>) -> Self {
		self.previous_response_id = Some(id.into());
		self
	}

	/// Enables streaming mode.
	pub fn with_stream(mut self, stream: bool) -> Self {
		self.stream = Some(stream);
		self
	}

	/// Sets the sampling temperature.
	pub fn with_temperature(mut self, temperature: f64) -> Self {
		self.temperature = Some(temperature);
		self
	}

	/// Adds a function tool to the request.
	pub fn with_tool(mut self, tool: FunctionToolParam) -> Self {
		self.tools.get_or_insert_with(Vec::new).push(tool);
		self
	}

	/// Sets the tool choice behavior.
	pub fn with_tool_choice(mut self, choice: ToolChoice) -> Self {
		self.tool_choice = Some(choice);
		self
	}

	/// Sets the maximum output tokens.
	pub fn with_max_output_tokens(mut self, max: u32) -> Self {
		self.max_output_tokens = Some(max);
		self
	}

	/// Sets additional instructions for the model.
	pub fn with_instructions(
		mut self,
		instructions: impl Into<String>,
	) -> Self {
		self.instructions = Some(instructions.into());
		self
	}

	/// Sets the reasoning configuration.
	pub fn with_reasoning(mut self, reasoning: ReasoningParam) -> Self {
		self.reasoning = Some(reasoning);
		self
	}
}

/// Input to the model - either a simple string or an array of input items.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Input {
	/// A simple string input, interpreted as a user message.
	Text(String),
	/// An array of input items for complex conversations.
	Items(Vec<InputItem>),
}

/// An input item in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputItem {
	/// A message item (user, assistant, system, or developer).
	Message(MessageParam),
	/// A reference to a previous item by ID.
	#[serde(rename = "item_reference")]
	ItemReference(ItemReference),
	/// A function call from the assistant.
	#[serde(rename = "function_call")]
	FunctionCall(FunctionCallParam),
	/// Output from a function call.
	#[serde(rename = "function_call_output")]
	FunctionCallOutput(FunctionCallOutputParam),
	/// A reasoning item.
	Reasoning(ReasoningItemParam),
}

/// A message input parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageParam {
	/// The unique ID of this message item.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<String>,
	/// The role of the message author.
	pub role: MessageRole,
	/// The message content.
	pub content: MessageContent,
	/// The status of the message item.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub status: Option<String>,
}

impl MessageParam {
	/// Creates a new user message with text content.
	pub fn user(content: impl Into<String>) -> Self {
		Self {
			id: None,
			role: MessageRole::User,
			content: MessageContent::Text(content.into()),
			status: None,
		}
	}

	/// Creates a new assistant message with text content.
	pub fn assistant(content: impl Into<String>) -> Self {
		Self {
			id: None,
			role: MessageRole::Assistant,
			content: MessageContent::Text(content.into()),
			status: None,
		}
	}

	/// Creates a new system message with text content.
	pub fn system(content: impl Into<String>) -> Self {
		Self {
			id: None,
			role: MessageRole::System,
			content: MessageContent::Text(content.into()),
			status: None,
		}
	}

	/// Creates a new developer message with text content.
	pub fn developer(content: impl Into<String>) -> Self {
		Self {
			id: None,
			role: MessageRole::Developer,
			content: MessageContent::Text(content.into()),
			status: None,
		}
	}

	/// Creates a new message with multimodal content parts.
	pub fn with_parts(role: MessageRole, parts: Vec<ContentPart>) -> Self {
		Self {
			id: None,
			role,
			content: MessageContent::Parts(parts),
			status: None,
		}
	}
}

/// Message content - either a simple string or an array of content parts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
	/// A simple text string.
	Text(String),
	/// An array of content parts for multimodal input.
	Parts(Vec<ContentPart>),
}

/// An item reference parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemReference {
	/// The ID of the item to reference.
	pub id: String,
}

impl ItemReference {
	/// Creates a new item reference.
	pub fn new(id: impl Into<String>) -> Self { Self { id: id.into() } }
}

/// A function call parameter (for providing previous function calls in input).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCallParam {
	/// The unique ID of this function tool call.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<String>,
	/// The unique ID of the function tool call generated by the model.
	pub call_id: String,
	/// The name of the function to call.
	pub name: String,
	/// The function arguments as a JSON string.
	pub arguments: String,
	/// The status of the function tool call.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub status: Option<FunctionCallStatus>,
}

/// A function call output parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCallOutputParam {
	/// The unique ID of the function tool call output.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<String>,
	/// The unique ID of the function tool call generated by the model.
	pub call_id: String,
	/// The output of the function tool call.
	pub output: FunctionOutputContent,
	/// The status of the item.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub status: Option<FunctionCallStatus>,
}

impl FunctionCallOutputParam {
	/// Creates a function call output with text.
	pub fn text(call_id: impl Into<String>, output: impl Into<String>) -> Self {
		Self {
			id: None,
			call_id: call_id.into(),
			output: FunctionOutputContent::Text(output.into()),
			status: None,
		}
	}

	/// Creates a function call output from a JSON-serializable value.
	pub fn json(
		call_id: impl Into<String>,
		output: &impl serde::Serialize,
	) -> Result<Self, serde_json::Error> {
		Ok(Self {
			id: None,
			call_id: call_id.into(),
			output: FunctionOutputContent::Text(serde_json::to_string(output)?),
			status: None,
		})
	}
}

/// Output content from a function call.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FunctionOutputContent {
	/// A text output.
	Text(String),
	/// An array of content parts.
	Parts(Vec<ContentPart>),
}

/// A reasoning item parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningItemParam {
	/// The unique ID of this reasoning item.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<String>,
	/// Reasoning summary content.
	pub summary: Vec<ReasoningSummaryContent>,
	/// Encrypted reasoning content for rehydration.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub encrypted_content: Option<String>,
}

/// Reasoning summary content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningSummaryContent {
	/// The content type. Always `"summary_text"`.
	#[serde(rename = "type")]
	pub content_type: String,
	/// The reasoning summary text.
	pub text: String,
}

impl ReasoningSummaryContent {
	/// Creates a new reasoning summary content.
	pub fn new(text: impl Into<String>) -> Self {
		Self {
			content_type: "summary_text".to_string(),
			text: text.into(),
		}
	}
}

/// Metadata key-value pairs.
pub type Metadata = std::collections::HashMap<String, String>;

/// Text output configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextParam {
	/// The format configuration for text output.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub format: Option<TextFormat>,
	/// Controls the level of detail in generated text output.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub verbosity: Option<Verbosity>,
}

/// Text format configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TextFormat {
	/// Plain text format.
	Text,
	/// JSON object format.
	#[serde(rename = "json_object")]
	JsonObject,
	/// JSON schema format.
	#[serde(rename = "json_schema")]
	JsonSchema(JsonSchemaFormat),
}

/// JSON schema format configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchemaFormat {
	/// The name of the response format.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,
	/// A description of what the response format is for.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	/// The schema for the response format as a JSON Schema object.
	pub schema: serde_json::Value,
	/// Whether to enable strict schema adherence.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub strict: Option<bool>,
}

/// Stream options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamOptions {
	/// Whether to obfuscate sensitive information in streamed output.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub include_obfuscation: Option<bool>,
}

/// Reasoning configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningParam {
	/// Controls the level of reasoning effort.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub effort: Option<ReasoningEffort>,
	/// Controls whether the response includes a reasoning summary.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub summary: Option<ReasoningSummary>,
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn serializes_simple_request() {
		let body = RequestBody::new("gpt-4o-mini").with_input("Hello");
		let json = serde_json::to_value(&body).unwrap();
		assert_eq!(json["model"], "gpt-4o-mini");
		assert_eq!(json["input"], "Hello");
	}

	#[test]
	fn serializes_message_items() {
		let body = RequestBody::new("gpt-4o-mini").with_input_items(vec![
			InputItem::Message(MessageParam::system("You are helpful.")),
			InputItem::Message(MessageParam::user("Hello")),
		]);
		let json = serde_json::to_value(&body).unwrap();
		assert_eq!(json["input"][0]["role"], "system");
		assert_eq!(json["input"][1]["role"], "user");
	}

	#[test]
	fn serializes_tool() {
		let tool = FunctionToolParam::new("get_weather")
			.with_description("Get the weather")
			.with_parameters(serde_json::json!({
				"type": "object",
				"properties": {
					"location": {"type": "string"}
				}
			}));
		let json = serde_json::to_value(&tool).unwrap();
		assert_eq!(json["type"], "function");
		assert_eq!(json["name"], "get_weather");
	}

	#[test]
	fn serializes_image_input() {
		let msg = MessageParam::with_parts(MessageRole::User, vec![
			ContentPart::input_text("What's in this image?"),
			ContentPart::input_image_url(
				"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==",
			),
		]);
		let json = serde_json::to_value(&msg).unwrap();
		assert_eq!(json["content"][0]["type"], "input_text");
		assert_eq!(json["content"][1]["type"], "input_image");
	}

	#[test]
	fn serializes_function_call_output() {
		let output =
			FunctionCallOutputParam::text("call_123", r#"{"temp": 72}"#);
		let json = serde_json::to_value(&output).unwrap();
		assert_eq!(json["call_id"], "call_123");
		assert_eq!(json["output"], r#"{"temp": 72}"#);
	}
}
