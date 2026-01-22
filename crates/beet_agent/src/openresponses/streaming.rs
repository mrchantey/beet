//! Server-Sent Event (SSE) streaming types for the OpenResponses API.
//!
//! When `stream: true` is set in a request, the API returns a stream of
//! Server-Sent Events that incrementally build up the response. This module
//! provides strongly-typed representations of all 24 streaming event types
//! defined in the [OpenResponses specification](https://www.openresponses.org/specification).
//!
//! # Streaming Lifecycle
//!
//! A typical streaming response follows this pattern:
//!
//! ```text
//! ┌─────────────────────┐
//! │ response.created    │  Response object initialized
//! └──────────┬──────────┘
//!            ▼
//! ┌─────────────────────┐
//! │ response.in_progress│  Generation begins
//! └──────────┬──────────┘
//!            ▼
//! ┌─────────────────────┐
//! │ output_item.added   │  Message/tool call started
//! └──────────┬──────────┘
//!            ▼
//! ┌─────────────────────┐
//! │ content_part.added  │  Content part (text/refusal) started
//! └──────────┬──────────┘
//!            ▼
//! ┌─────────────────────┐
//! │ output_text.delta   │  Incremental text chunks (repeated)
//! └──────────┬──────────┘
//!            ▼
//! ┌─────────────────────┐
//! │ output_text.done    │  Text content finalized
//! └──────────┬──────────┘
//!            ▼
//! ┌─────────────────────┐
//! │ content_part.done   │  Content part finalized
//! └──────────┬──────────┘
//!            ▼
//! ┌─────────────────────┐
//! │ output_item.done    │  Message/tool call finalized
//! └──────────┬──────────┘
//!            ▼
//! ┌─────────────────────┐
//! │ response.completed  │  Full response ready
//! └─────────────────────┘
//! ```
//!
//! # Event Categories
//!
//! Events are organized into several categories:
//!
//! - **Response lifecycle**: `created`, `queued`, `in_progress`, `completed`, `failed`, `incomplete`
//! - **Output items**: `output_item.added`, `output_item.done`
//! - **Content parts**: `content_part.added`, `content_part.done`
//! - **Text streaming**: `output_text.delta`, `output_text.done`, `output_text.annotation.added`
//! - **Refusal streaming**: `refusal.delta`, `refusal.done`
//! - **Reasoning**: `reasoning.delta`, `reasoning.done`, `reasoning_summary_text.delta/done`, `reasoning_summary_part.added/done`
//! - **Function calls**: `function_call_arguments.delta`, `function_call_arguments.done`
//! - **Errors**: `error`
//!
//! # Example
//!
//! ```no_run
//! use beet_agent::prelude::*;
//! use beet_core::prelude::*;
//! use beet_net::prelude::*;
//!
//! # async fn example() -> Result<()> {
//! let mut provider = OllamaProvider::default();
//!
//! let body = openresponses::RequestBody::new(provider.default_small_model())
//!     .with_input("Write a haiku about streaming.")
//!     .with_stream(true);
//!
//! let mut stream = provider.stream(body).await?;
//!
//! while let Some(event) = stream.next().await {
//!     match event? {
//!         openresponses::StreamingEvent::OutputTextDelta(ev) => {
//!             print!("{}", ev.delta);
//!         }
//!         openresponses::StreamingEvent::ResponseCompleted(ev) => {
//!             println!("\n\nDone! Used {} tokens", ev.response.usage.map(|u| u.total_tokens).unwrap_or(0));
//!         }
//!         _ => {}
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use super::*;
use serde::Deserialize;
use serde::Serialize;

/// A streaming event from the OpenResponses API.
///
/// This enum represents all 24 streaming event types that can be received
/// when `stream: true` is set in the request. Events are discriminated by
/// their `type` field in the JSON payload.
///
/// # Wire Format
///
/// Events arrive as Server-Sent Events (SSE) with the following format:
/// ```text
/// event: response.output_text.delta
/// data: {"type":"response.output_text.delta","sequence_number":5,...}
/// ```
///
/// The special `[DONE]` message indicates the stream has ended and should
/// be handled separately before attempting to parse as a `StreamingEvent`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamingEvent {
	// ═══════════════════════════════════════════════════════════════════════
	// Response Lifecycle Events
	// ═══════════════════════════════════════════════════════════════════════
	/// Emitted when the response object is first created.
	///
	/// This is typically the first event in a stream and contains
	/// the initial response snapshot with status `in_progress`.
	#[serde(rename = "response.created")]
	ResponseCreated(ResponseCreatedEvent),

	/// Emitted when the response is queued for processing.
	///
	/// This event indicates the request has been accepted and is
	/// waiting for model capacity. Common with background requests.
	#[serde(rename = "response.queued")]
	ResponseQueued(ResponseQueuedEvent),

	/// Emitted when response generation actively begins.
	///
	/// The response transitions from queued/created to actively
	/// generating output.
	#[serde(rename = "response.in_progress")]
	ResponseInProgress(ResponseInProgressEvent),

	/// Emitted when the response completes successfully.
	///
	/// The response snapshot contains all generated output items
	/// and final usage statistics.
	#[serde(rename = "response.completed")]
	ResponseCompleted(ResponseCompletedEvent),

	/// Emitted when the response fails due to an error.
	///
	/// The response will contain an `error` field with details.
	#[serde(rename = "response.failed")]
	ResponseFailed(ResponseFailedEvent),

	/// Emitted when the response is incomplete.
	///
	/// This can occur due to max tokens, content filtering, or
	/// other interruptions. Check `incomplete_details` for the reason.
	#[serde(rename = "response.incomplete")]
	ResponseIncomplete(ResponseIncompleteEvent),

	// ═══════════════════════════════════════════════════════════════════════
	// Output Item Events
	// ═══════════════════════════════════════════════════════════════════════
	/// Emitted when a new output item (message, function call, etc.) is added.
	///
	/// The `item` field contains the initial item state, typically with
	/// empty content that will be populated by subsequent delta events.
	#[serde(rename = "response.output_item.added")]
	OutputItemAdded(OutputItemAddedEvent),

	/// Emitted when an output item is finalized.
	///
	/// The `item` field contains the complete item with all content.
	#[serde(rename = "response.output_item.done")]
	OutputItemDone(OutputItemDoneEvent),

	// ═══════════════════════════════════════════════════════════════════════
	// Content Part Events
	// ═══════════════════════════════════════════════════════════════════════
	/// Emitted when a new content part is added to a message.
	///
	/// Content parts include text outputs, refusals, and other content types.
	#[serde(rename = "response.content_part.added")]
	ContentPartAdded(ContentPartAddedEvent),

	/// Emitted when a content part is finalized.
	#[serde(rename = "response.content_part.done")]
	ContentPartDone(ContentPartDoneEvent),

	// ═══════════════════════════════════════════════════════════════════════
	// Text Streaming Events
	// ═══════════════════════════════════════════════════════════════════════
	/// Emitted incrementally as text is generated.
	///
	/// Accumulate the `delta` field to build the complete text.
	/// This is the most frequent event type during text generation.
	#[serde(rename = "response.output_text.delta")]
	OutputTextDelta(OutputTextDeltaEvent),

	/// Emitted when text generation for a content part completes.
	///
	/// The `text` field contains the complete generated text.
	#[serde(rename = "response.output_text.done")]
	OutputTextDone(OutputTextDoneEvent),

	/// Emitted when an annotation (citation, etc.) is added to text.
	#[serde(rename = "response.output_text.annotation.added")]
	OutputTextAnnotationAdded(OutputTextAnnotationAddedEvent),

	// ═══════════════════════════════════════════════════════════════════════
	// Refusal Events
	// ═══════════════════════════════════════════════════════════════════════
	/// Emitted incrementally when the model generates a refusal.
	#[serde(rename = "response.refusal.delta")]
	RefusalDelta(RefusalDeltaEvent),

	/// Emitted when refusal generation completes.
	#[serde(rename = "response.refusal.done")]
	RefusalDone(RefusalDoneEvent),

	// ═══════════════════════════════════════════════════════════════════════
	// Reasoning Events (for reasoning models like o1, o3)
	// ═══════════════════════════════════════════════════════════════════════
	/// Emitted incrementally as reasoning text is generated.
	#[serde(rename = "response.reasoning.delta")]
	ReasoningDelta(ReasoningDeltaEvent),

	/// Emitted when reasoning generation completes.
	#[serde(rename = "response.reasoning.done")]
	ReasoningDone(ReasoningDoneEvent),

	/// Emitted incrementally as reasoning summary text is generated.
	#[serde(rename = "response.reasoning_summary_text.delta")]
	ReasoningSummaryTextDelta(ReasoningSummaryTextDeltaEvent),

	/// Emitted when reasoning summary text completes.
	#[serde(rename = "response.reasoning_summary_text.done")]
	ReasoningSummaryTextDone(ReasoningSummaryTextDoneEvent),

	/// Emitted when a reasoning summary part is added.
	#[serde(rename = "response.reasoning_summary_part.added")]
	ReasoningSummaryPartAdded(ReasoningSummaryPartAddedEvent),

	/// Emitted when a reasoning summary part completes.
	#[serde(rename = "response.reasoning_summary_part.done")]
	ReasoningSummaryPartDone(ReasoningSummaryPartDoneEvent),

	// ═══════════════════════════════════════════════════════════════════════
	// Function Call Events
	// ═══════════════════════════════════════════════════════════════════════
	/// Emitted incrementally as function call arguments are generated.
	///
	/// Accumulate the `delta` field to build the complete arguments JSON.
	#[serde(rename = "response.function_call_arguments.delta")]
	FunctionCallArgumentsDelta(FunctionCallArgumentsDeltaEvent),

	/// Emitted when function call arguments generation completes.
	///
	/// The `arguments` field contains the complete JSON arguments string.
	#[serde(rename = "response.function_call_arguments.done")]
	FunctionCallArgumentsDone(FunctionCallArgumentsDoneEvent),

	// ═══════════════════════════════════════════════════════════════════════
	// Error Events
	// ═══════════════════════════════════════════════════════════════════════
	/// Emitted when an error occurs during streaming.
	///
	/// Note: This is different from `response.failed` which indicates
	/// the response itself failed. This `error` event can occur for
	/// stream-level issues.
	Error(ErrorEvent),
}

impl StreamingEvent {
	/// Returns the sequence number of this event.
	///
	/// Sequence numbers increase monotonically and can be used to
	/// detect missed events or ensure proper ordering.
	pub fn sequence_number(&self) -> i64 {
		match self {
			Self::ResponseCreated(ev) => ev.sequence_number,
			Self::ResponseQueued(ev) => ev.sequence_number,
			Self::ResponseInProgress(ev) => ev.sequence_number,
			Self::ResponseCompleted(ev) => ev.sequence_number,
			Self::ResponseFailed(ev) => ev.sequence_number,
			Self::ResponseIncomplete(ev) => ev.sequence_number,
			Self::OutputItemAdded(ev) => ev.sequence_number,
			Self::OutputItemDone(ev) => ev.sequence_number,
			Self::ContentPartAdded(ev) => ev.sequence_number,
			Self::ContentPartDone(ev) => ev.sequence_number,
			Self::OutputTextDelta(ev) => ev.sequence_number,
			Self::OutputTextDone(ev) => ev.sequence_number,
			Self::OutputTextAnnotationAdded(ev) => ev.sequence_number,
			Self::RefusalDelta(ev) => ev.sequence_number,
			Self::RefusalDone(ev) => ev.sequence_number,
			Self::ReasoningDelta(ev) => ev.sequence_number,
			Self::ReasoningDone(ev) => ev.sequence_number,
			Self::ReasoningSummaryTextDelta(ev) => ev.sequence_number,
			Self::ReasoningSummaryTextDone(ev) => ev.sequence_number,
			Self::ReasoningSummaryPartAdded(ev) => ev.sequence_number,
			Self::ReasoningSummaryPartDone(ev) => ev.sequence_number,
			Self::FunctionCallArgumentsDelta(ev) => ev.sequence_number,
			Self::FunctionCallArgumentsDone(ev) => ev.sequence_number,
			Self::Error(ev) => ev.sequence_number,
		}
	}

	/// Returns the event type string as it appears in the wire format.
	pub fn event_type(&self) -> &'static str {
		match self {
			Self::ResponseCreated(_) => "response.created",
			Self::ResponseQueued(_) => "response.queued",
			Self::ResponseInProgress(_) => "response.in_progress",
			Self::ResponseCompleted(_) => "response.completed",
			Self::ResponseFailed(_) => "response.failed",
			Self::ResponseIncomplete(_) => "response.incomplete",
			Self::OutputItemAdded(_) => "response.output_item.added",
			Self::OutputItemDone(_) => "response.output_item.done",
			Self::ContentPartAdded(_) => "response.content_part.added",
			Self::ContentPartDone(_) => "response.content_part.done",
			Self::OutputTextDelta(_) => "response.output_text.delta",
			Self::OutputTextDone(_) => "response.output_text.done",
			Self::OutputTextAnnotationAdded(_) => {
				"response.output_text.annotation.added"
			}
			Self::RefusalDelta(_) => "response.refusal.delta",
			Self::RefusalDone(_) => "response.refusal.done",
			Self::ReasoningDelta(_) => "response.reasoning.delta",
			Self::ReasoningDone(_) => "response.reasoning.done",
			Self::ReasoningSummaryTextDelta(_) => {
				"response.reasoning_summary_text.delta"
			}
			Self::ReasoningSummaryTextDone(_) => {
				"response.reasoning_summary_text.done"
			}
			Self::ReasoningSummaryPartAdded(_) => {
				"response.reasoning_summary_part.added"
			}
			Self::ReasoningSummaryPartDone(_) => {
				"response.reasoning_summary_part.done"
			}
			Self::FunctionCallArgumentsDelta(_) => {
				"response.function_call_arguments.delta"
			}
			Self::FunctionCallArgumentsDone(_) => {
				"response.function_call_arguments.done"
			}
			Self::Error(_) => "error",
		}
	}
}

// ═══════════════════════════════════════════════════════════════════════════
// Response Lifecycle Event Types
// ═══════════════════════════════════════════════════════════════════════════

/// Event indicating the response was created.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseCreatedEvent {
	/// The sequence number of this event.
	pub sequence_number: i64,
	/// The response snapshot at creation time.
	pub response: response::Body,
}

/// Event indicating the response was queued.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseQueuedEvent {
	/// The sequence number of this event.
	pub sequence_number: i64,
	/// The response snapshot.
	pub response: response::Body,
}

/// Event indicating the response is in progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseInProgressEvent {
	/// The sequence number of this event.
	pub sequence_number: i64,
	/// The response snapshot.
	pub response: response::Body,
}

/// Event indicating the response completed successfully.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseCompletedEvent {
	/// The sequence number of this event.
	pub sequence_number: i64,
	/// The final response with all output and usage data.
	pub response: response::Body,
}

/// Event indicating the response failed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseFailedEvent {
	/// The sequence number of this event.
	pub sequence_number: i64,
	/// The response snapshot containing error details.
	pub response: response::Body,
}

/// Event indicating the response was incomplete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseIncompleteEvent {
	/// The sequence number of this event.
	pub sequence_number: i64,
	/// The response snapshot with incomplete_details.
	pub response: response::Body,
}

// ═══════════════════════════════════════════════════════════════════════════
// Output Item Event Types
// ═══════════════════════════════════════════════════════════════════════════

/// Event indicating an output item was added.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputItemAddedEvent {
	/// The sequence number of this event.
	pub sequence_number: i64,
	/// The index of the output item in the response's output array.
	pub output_index: u32,
	/// The output item that was added.
	pub item: Option<OutputItem>,
}

/// Event indicating an output item is complete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputItemDoneEvent {
	/// The sequence number of this event.
	pub sequence_number: i64,
	/// The index of the output item.
	pub output_index: u32,
	/// The finalized output item.
	pub item: Option<OutputItem>,
}

// ═══════════════════════════════════════════════════════════════════════════
// Content Part Event Types
// ═══════════════════════════════════════════════════════════════════════════

/// Event indicating a content part was added.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPartAddedEvent {
	/// The sequence number of this event.
	pub sequence_number: i64,
	/// The ID of the item being updated.
	pub item_id: String,
	/// The index of the output item.
	pub output_index: u32,
	/// The index of the content part within the item.
	pub content_index: u32,
	/// The content part that was added.
	pub part: ContentPart,
}

/// Event indicating a content part is complete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPartDoneEvent {
	/// The sequence number of this event.
	pub sequence_number: i64,
	/// The ID of the item being updated.
	pub item_id: String,
	/// The index of the output item.
	pub output_index: u32,
	/// The index of the content part.
	pub content_index: u32,
	/// The finalized content part.
	pub part: ContentPart,
}

// ═══════════════════════════════════════════════════════════════════════════
// Text Streaming Event Types
// ═══════════════════════════════════════════════════════════════════════════

/// Event containing incremental text output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputTextDeltaEvent {
	/// The sequence number of this event.
	pub sequence_number: i64,
	/// The ID of the item being updated.
	pub item_id: String,
	/// The index of the output item.
	pub output_index: u32,
	/// The index of the content part.
	pub content_index: u32,
	/// The text delta to append.
	pub delta: String,
	/// Token log probabilities, if requested.
	#[serde(default)]
	pub logprobs: Vec<LogProb>,
	/// Obfuscation padding string, if enabled.
	#[serde(default)]
	pub obfuscation: Option<String>,
}

/// Event indicating text output is complete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputTextDoneEvent {
	/// The sequence number of this event.
	pub sequence_number: i64,
	/// The ID of the item being updated.
	pub item_id: String,
	/// The index of the output item.
	pub output_index: u32,
	/// The index of the content part.
	pub content_index: u32,
	/// The complete text.
	pub text: String,
	/// Token log probabilities for the complete text, if requested.
	#[serde(default)]
	pub logprobs: Vec<LogProb>,
}

/// Event indicating a text annotation was added.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputTextAnnotationAddedEvent {
	/// The sequence number of this event.
	pub sequence_number: i64,
	/// The ID of the item being updated.
	pub item_id: String,
	/// The index of the output item.
	pub output_index: u32,
	/// The index of the content part.
	pub content_index: u32,
	/// The index of the annotation.
	pub annotation_index: u32,
	/// The annotation that was added.
	pub annotation: Option<Annotation>,
}

// ═══════════════════════════════════════════════════════════════════════════
// Refusal Event Types
// ═══════════════════════════════════════════════════════════════════════════

/// Event containing incremental refusal text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefusalDeltaEvent {
	/// The sequence number of this event.
	pub sequence_number: i64,
	/// The ID of the item being updated.
	pub item_id: String,
	/// The index of the output item.
	pub output_index: u32,
	/// The index of the content part.
	pub content_index: u32,
	/// The refusal text delta.
	pub delta: String,
}

/// Event indicating refusal is complete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefusalDoneEvent {
	/// The sequence number of this event.
	pub sequence_number: i64,
	/// The ID of the item being updated.
	pub item_id: String,
	/// The index of the output item.
	pub output_index: u32,
	/// The index of the content part.
	pub content_index: u32,
	/// The complete refusal text.
	pub refusal: String,
}

// ═══════════════════════════════════════════════════════════════════════════
// Reasoning Event Types
// ═══════════════════════════════════════════════════════════════════════════

/// Event containing incremental reasoning text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningDeltaEvent {
	/// The sequence number of this event.
	pub sequence_number: i64,
	/// The ID of the item being updated.
	pub item_id: String,
	/// The index of the output item.
	pub output_index: u32,
	/// The index of the content part.
	pub content_index: u32,
	/// The reasoning text delta.
	pub delta: String,
	/// Obfuscation padding string, if enabled.
	#[serde(default)]
	pub obfuscation: Option<String>,
}

/// Event indicating reasoning is complete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningDoneEvent {
	/// The sequence number of this event.
	pub sequence_number: i64,
	/// The ID of the item being updated.
	pub item_id: String,
	/// The index of the output item.
	pub output_index: u32,
	/// The index of the content part.
	pub content_index: u32,
	/// The complete reasoning text.
	pub text: String,
}

/// Event containing incremental reasoning summary text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningSummaryTextDeltaEvent {
	/// The sequence number of this event.
	pub sequence_number: i64,
	/// The ID of the item being updated.
	pub item_id: String,
	/// The index of the output item.
	pub output_index: u32,
	/// The index of the summary content.
	/// Optional for provider compatibility (some providers omit this).
	#[serde(default)]
	pub summary_index: Option<u32>,
	/// The summary text delta.
	pub delta: String,
	/// Obfuscation padding string, if enabled.
	#[serde(default)]
	pub obfuscation: Option<String>,
}

/// Event indicating reasoning summary text is complete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningSummaryTextDoneEvent {
	/// The sequence number of this event.
	pub sequence_number: i64,
	/// The ID of the item being updated.
	pub item_id: String,
	/// The index of the output item.
	pub output_index: u32,
	/// The index of the summary content.
	/// Optional for provider compatibility (some providers omit this).
	#[serde(default)]
	pub summary_index: Option<u32>,
	/// The complete summary text.
	pub text: String,
}

/// Event indicating a reasoning summary part was added.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningSummaryPartAddedEvent {
	/// The sequence number of this event.
	pub sequence_number: i64,
	/// The ID of the item being updated.
	pub item_id: String,
	/// The index of the output item.
	pub output_index: u32,
	/// The index of the summary part.
	/// Optional for provider compatibility (some providers omit this).
	#[serde(default)]
	pub summary_index: Option<u32>,
	/// The summary part that was added.
	pub part: ContentPart,
}

/// Event indicating a reasoning summary part is complete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningSummaryPartDoneEvent {
	/// The sequence number of this event.
	pub sequence_number: i64,
	/// The ID of the item being updated.
	pub item_id: String,
	/// The index of the output item.
	pub output_index: u32,
	/// The index of the summary part.
	/// Optional for provider compatibility (some providers omit this).
	#[serde(default)]
	pub summary_index: Option<u32>,
	/// The finalized summary part.
	pub part: ContentPart,
}

// ═══════════════════════════════════════════════════════════════════════════
// Function Call Event Types
// ═══════════════════════════════════════════════════════════════════════════

/// Event containing incremental function call arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCallArgumentsDeltaEvent {
	/// The sequence number of this event.
	pub sequence_number: i64,
	/// The ID of the function call item.
	pub item_id: String,
	/// The index of the output item.
	pub output_index: u32,
	/// The arguments JSON delta.
	pub delta: String,
	/// Obfuscation padding string, if enabled.
	#[serde(default)]
	pub obfuscation: Option<String>,
}

/// Event indicating function call arguments are complete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCallArgumentsDoneEvent {
	/// The sequence number of this event.
	pub sequence_number: i64,
	/// The ID of the function call item.
	pub item_id: String,
	/// The index of the output item.
	pub output_index: u32,
	/// The complete arguments JSON string.
	pub arguments: String,
}

// ═══════════════════════════════════════════════════════════════════════════
// Error Event Types
// ═══════════════════════════════════════════════════════════════════════════

/// Event indicating an error occurred during streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEvent {
	/// The sequence number of this event.
	pub sequence_number: i64,
	/// The error payload.
	pub error: ErrorPayload,
}

/// Error details in an error event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPayload {
	/// The error type.
	#[serde(rename = "type")]
	pub error_type: String,
	/// A machine-readable error code, if available.
	pub code: Option<String>,
	/// A human-readable error message.
	pub message: String,
	/// The parameter name associated with the error, if applicable.
	pub param: Option<String>,
	/// Response headers associated with the error.
	#[serde(default)]
	pub headers: std::collections::HashMap<String, String>,
}


#[cfg(test)]
mod test {
	use super::*;
	use beet_core::prelude::*;

	#[test]
	fn deserializes_response_created() {
		let json = r#"{
			"type": "response.created",
			"sequence_number": 0,
			"response": {
				"id": "resp_123",
				"object": "response",
				"created_at": 1700000000,
				"status": "in_progress",
				"model": "gpt-4o-mini",
				"output": [],
				"tools": []
			}
		}"#;

		let event: StreamingEvent = serde_json::from_str(json).unwrap();
		matches!(event, StreamingEvent::ResponseCreated(_)).xpect_true();
		event.sequence_number().xpect_eq(0);
		event.event_type().xpect_eq("response.created");
	}

	#[test]
	fn deserializes_output_text_delta() {
		let json = r#"{
			"type": "response.output_text.delta",
			"sequence_number": 5,
			"item_id": "msg_123",
			"output_index": 0,
			"content_index": 0,
			"delta": "Hello",
			"logprobs": []
		}"#;

		let event: StreamingEvent = serde_json::from_str(json).unwrap();
		if let StreamingEvent::OutputTextDelta(ev) = &event {
			ev.delta.xpect_eq("Hello");
			ev.item_id.xpect_eq("msg_123");
		} else {
			panic!("Expected OutputTextDelta event");
		}
	}

	#[test]
	fn deserializes_function_call_arguments_done() {
		let json = r#"{
			"type": "response.function_call_arguments.done",
			"sequence_number": 10,
			"item_id": "fc_123",
			"output_index": 0,
			"arguments": "{\"location\":\"NYC\"}"
		}"#;

		let event: StreamingEvent = serde_json::from_str(json).unwrap();
		if let StreamingEvent::FunctionCallArgumentsDone(ev) = &event {
			ev.arguments.xpect_eq("{\"location\":\"NYC\"}");
		} else {
			panic!("Expected FunctionCallArgumentsDone event");
		}
	}

	#[test]
	fn deserializes_error_event() {
		let json = r#"{
			"type": "error",
			"sequence_number": 1,
			"error": {
				"type": "invalid_request_error",
				"code": "invalid_api_key",
				"message": "Invalid API key provided",
				"param": null
			}
		}"#;

		let event: StreamingEvent = serde_json::from_str(json).unwrap();
		if let StreamingEvent::Error(ev) = &event {
			ev.error.error_type.xpect_eq("invalid_request_error");
			ev.error.message.xpect_eq("Invalid API key provided");
		} else {
			panic!("Expected Error event");
		}
	}

	#[test]
	fn deserializes_response_completed() {
		let json = r#"{
			"type": "response.completed",
			"sequence_number": 15,
			"response": {
				"id": "resp_456",
				"object": "response",
				"created_at": 1700000000,
				"completed_at": 1700000005,
				"status": "completed",
				"model": "gpt-4o-mini",
				"output": [
					{
						"type": "message",
						"id": "msg_789",
						"status": "completed",
						"role": "assistant",
						"content": [
							{
								"type": "output_text",
								"text": "Hello, world!",
								"annotations": [],
								"logprobs": []
							}
						]
					}
				],
				"tools": [],
				"usage": {
					"input_tokens": 10,
					"output_tokens": 5,
					"total_tokens": 15,
					"input_tokens_details": {"cached_tokens": 0},
					"output_tokens_details": {"reasoning_tokens": 0}
				}
			}
		}"#;

		let event: StreamingEvent = serde_json::from_str(json).unwrap();
		if let StreamingEvent::ResponseCompleted(ev) = &event {
			ev.response.status.xpect_eq(response::Status::Completed);
			ev.response.first_text().xpect_eq(Some("Hello, world!"));
		} else {
			panic!("Expected ResponseCompleted event");
		}
	}
}
