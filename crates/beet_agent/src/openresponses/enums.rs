//! Common enums used across the OpenResponses API.

use serde::Deserialize;
use serde::Serialize;

/// Role of a message in the conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
	/// End-user input in the conversation.
	User,
	/// Model-generated content.
	Assistant,
	/// System-level instructions that set global behavior.
	System,
	/// Developer-supplied guidance that shapes the assistant's behavior.
	Developer,
}

/// Status of a message item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageStatus {
	/// Model is currently sampling this item.
	InProgress,
	/// Model has finished sampling this item.
	Completed,
	/// Model was interrupted partway through.
	Incomplete,
}

/// Status of a function call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FunctionCallStatus {
	/// Model is currently sampling this item.
	InProgress,
	/// Model has finished sampling this item.
	Completed,
	/// Model was interrupted partway through.
	Incomplete,
}

/// Image detail level for vision inputs.
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ImageDetail {
	/// Lower-resolution version of the image.
	Low,
	/// Higher-resolution version of the image (may increase token costs).
	High,
	/// Choose the detail level automatically.
	#[default]
	Auto,
}

/// Reasoning effort level for reasoning models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningEffort {
	/// No reasoning before emitting a final answer.
	None,
	/// Lower reasoning effort for faster responses.
	Low,
	/// Balanced reasoning effort.
	Medium,
	/// Higher reasoning effort for improved quality.
	High,
	/// Maximum reasoning effort available.
	Xhigh,
}

/// Reasoning summary options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningSummary {
	/// Emit concise summaries.
	Concise,
	/// Emit detailed summaries.
	Detailed,
	/// Allow the model to decide when to summarize.
	Auto,
}

/// Service tier options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceTier {
	/// Choose automatically based on account state.
	Auto,
	/// Default service tier.
	Default,
	/// Flex service tier.
	Flex,
	/// Priority service tier.
	Priority,
}

/// Truncation options for context window management.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Truncation {
	/// Let the service decide how to truncate.
	Auto,
	/// Disable service truncation. Context over the model's limit will result in an error.
	Disabled,
}

/// Verbosity level for text output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verbosity {
	/// Less verbose final responses.
	Low,
	/// Default verbosity.
	Medium,
	/// More verbose final responses.
	High,
}

/// Simple tool choice values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolChoiceValue {
	/// Restrict the model from calling any tools.
	None,
	/// Let the model choose from the provided tools.
	Auto,
	/// Require the model to call a tool.
	Required,
}

/// Include options for additional response data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IncludeOption {
	/// Include encrypted reasoning content for rehydration.
	#[serde(rename = "reasoning.encrypted_content")]
	ReasoningEncryptedContent,
	/// Include sampled logprobs in assistant messages.
	#[serde(rename = "message.output_text.logprobs")]
	MessageOutputTextLogprobs,
}
