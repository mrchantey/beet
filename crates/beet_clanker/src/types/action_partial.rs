use crate::openresponses::Annotation;
use crate::openresponses::ContentPart;
use crate::openresponses::FunctionCallStatus;
use crate::openresponses::LogProb;
use crate::openresponses::MessageStatus;
use crate::openresponses::OutputContent;
use crate::openresponses::OutputItem;
use crate::prelude::*;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;


/// Items come from model providers in all sorts of weird and wonderful ways.
/// The first step is to translate these forms into a unified type,
/// for integrating into our own stateful representations.
/// The [`key`] and [`status`] fields are particularly useful for diffing
/// state.
///
///
/// openresponses ----> PartialItem ----> Action
///
/// ## Duplicate Events
///
/// This type will reduce many matching events into an identical representation,
/// for example OutputTextDone and OutputItemDone, which is intended
/// as an equality check is used before reifying into an [`Action`].
///
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionPartial {
	pub key: ActionPartialKey,
	pub status: ActionStatus,
	pub content: PartialContent,
}

impl ActionPartial {
	pub fn from_output_items(
		items: impl IntoIterator<Item = OutputItem>,
		status: ActionStatus,
	) -> impl IntoIterator<Item = ActionPartial> {
		items
			.into_iter()
			.flat_map(move |item| Self::from_output_item(item, status))
	}
	/// Create a [`Vec<PartialItem>`] from an output item,
	/// optionally setting status for ambiguous types like function call items.
	///
	/// ## `status`
	/// The status field is used for variants where the status is optional:
	/// - [`OutputItem::FunctionCall`]
	/// - [`OutputItem::Reasoning`]
	/// defaulting to [`ActionStatus::Completed`]
	pub fn from_output_item(
		item: OutputItem,
		status: ActionStatus,
	) -> Vec<Self> {
		match item {
			OutputItem::Message(message) => {
				let status = match message.status {
					MessageStatus::InProgress => ActionStatus::InProgress,
					MessageStatus::Completed => ActionStatus::Completed,
					MessageStatus::Incomplete => ActionStatus::Interrupted,
				};
				message
					.content
					.into_iter()
					.enumerate()
					.map(|(content_index, content)| {
						let key = ActionPartialKey::Content {
							responses_id: message.id.clone(),
							content_index: content_index as u32,
						};
						ActionPartial {
							key,
							status,
							content: PartialContent::OutputContent(content),
						}
					})
					.collect()
			}
			OutputItem::FunctionCall(fc_call) => {
				let status = fc_call
					.status
					.map(|status| match status {
						FunctionCallStatus::InProgress => {
							ActionStatus::InProgress
						}
						FunctionCallStatus::Completed => {
							ActionStatus::Completed
						}
						FunctionCallStatus::Incomplete => {
							ActionStatus::Interrupted
						}
					})
					.unwrap_or(status);
				vec![ActionPartial {
					key: ActionPartialKey::Single {
						responses_id: fc_call.id,
					},
					status,
					content: PartialContent::FunctionCall {
						name: fc_call.name,
						call_id: fc_call.call_id,
						arguments: fc_call.arguments,
					},
				}]
			}
			OutputItem::FunctionCallOutput(fc_output) => {
				let status = match fc_output.status {
					FunctionCallStatus::InProgress => ActionStatus::InProgress,
					FunctionCallStatus::Completed => ActionStatus::Completed,
					FunctionCallStatus::Incomplete => ActionStatus::Interrupted,
				};
				vec![ActionPartial {
					key: ActionPartialKey::Single {
						responses_id: fc_output.id,
					},
					status,
					content: PartialContent::FunctionCallOutput {
						call_id: fc_output.call_id,
						output: fc_output.output,
					},
				}]
			}
			OutputItem::Reasoning(reasoning_item) => {
				let mut out = Vec::with_capacity(
					reasoning_item.content.len() + reasoning_item.summary.len(),
				);
				for (index, content) in
					reasoning_item.content.into_iter().enumerate()
				{
					out.push(ActionPartial {
						key: ActionPartialKey::Content {
							responses_id: reasoning_item.id.clone(),
							content_index: index as u32,
						},
						status,
						content: PartialContent::ReasoningContent(content.text),
					});
				}
				for (index, summary) in
					reasoning_item.summary.into_iter().enumerate()
				{
					out.push(ActionPartial {
						key: ActionPartialKey::ReasoningSummary {
							responses_id: reasoning_item.id.clone(),
							summary_index: index as u32,
						},
						status,
						content: PartialContent::ReasoningSummary(summary.text),
					});
				}
				out
			}
		}
	}
	pub fn from_delta(
		responses_id: String,
		content_index: u32,
		delta: String,
	) -> Self {
		Self {
			key: ActionPartialKey::Content {
				responses_id,
				content_index,
			},
			status: ActionStatus::InProgress,
			content: PartialContent::Delta(delta),
		}
	}
}


#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum ActionPartialKey {
	/// There is only one piece of content, ie a function call
	Single { responses_id: String },
	/// The item has multiple pieces of content, ie text, reasoning
	Content {
		responses_id: String,
		content_index: u32,
	},
	/// Reasoning summary is a special case that shares the same
	/// item id as content.
	ReasoningSummary {
		responses_id: String,
		// defaults to 0 when omitted by streaming
		summary_index: u32,
	},
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PartialContent {
	/// [`OutputItem::Message`]
	OutputContent(OutputContent),
	ContentPart(ContentPart),
	/// - [`OutputItem::FunctionCall`]
	FunctionCall {
		name: String,
		call_id: String,
		arguments: String,
	},
	/// - [`OutputItem::FunctionCallOutput`]
	FunctionCallOutput {
		call_id: String,
		output: String,
	},
	/// - [`OutputItem::Reasoning`] - content
	ReasoningContent(String),
	/// - [`OutputItem::Reasoning`] - summary
	/// - [`StreamingEvent::ReasoningSummary`]
	ReasoningSummary(String),
	/// - [`StreamingEvent::OutputTextDelta`]
	/// - [`StreamingEvent::RefusalDelta`]
	/// - [`StreamingEvent::ReasoningDelta`]
	/// - [`StreamingEvent::ReasoningSummaryTextDelta`]
	/// - [`StreamingEvent::FunctionCallArgumentsDelta`]
	Delta(String),
	/// - [`StreamingEvent::OutputTextDone`]
	TextDone {
		text: String,
		logprobs: Vec<LogProb>,
	},
	/// - [`StreamingEvent::RefusalDone`]
	RefusalDone {
		refusal: String,
	},
	/// - [`StreamingEvent::ReasoningDone`]
	ReasoningDone {
		content: String,
	},
	/// - [`StreamingEvent::OutputTextAnnotationAdded`]
	AnnotationAdded {
		annotation_index: u32,
		annotation: Annotation,
	},
	/// - [`StreamingEvent::FunctionCallArgumentsDone`]
	/// Full arguments are provided but name and call_id
	/// are not.
	/// The function call must already exist and its arguments
	/// will be overwritten with this string.
	FunctionCallArgumentsDone(String),
}

