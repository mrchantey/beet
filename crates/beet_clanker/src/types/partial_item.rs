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


#[derive(Debug, Default)]
pub struct PartialItemMap {
	/// Cache of partial items that have been parsed,
	/// used for comparison against incoming items to
	/// determine if they are unique.
	partial_items: HashMap<PartialItemKey, PartialItem>,
	item_key_map: HashMap<PartialItemKey, ItemId>,
}

impl PartialItemMap {
	pub fn apply_items(
		&mut self,
		item_map: &mut DocMap<Item>,
		owner: ActorId,
		partial_items: impl IntoIterator<Item = PartialItem>,
		// ) -> impl Iterator<Item = Item> {
	) -> Result<ApplyItemsOutcome> {
		let mut outcomes = ApplyItemsOutcome::default();
		for partial_item in partial_items {
			if let Some(other) = self.partial_items.get(&partial_item.key)
				&& other == &partial_item
			{
				// no change in the partial item, break
				continue;
			} else if let Some(&item_id) =
				self.item_key_map.get(&partial_item.key)
			{
				// item exists, mutate it.
				let item = item_map.get_mut(item_id)?;
				item.set_status(partial_item.status);
				partial_item.content.apply(item.content_mut())?;
				outcomes.modified.push(item_id);
			} else {
				// create new item
				let content = partial_item.content.into_content()?;
				let item = Item::new(owner, partial_item.status, content);
				outcomes.created.push(item.id());
				item_map.insert(item);
			}
		}
		outcomes.xok()
	}
}

#[derive(Default)]
pub struct ApplyItemsOutcome {
	created: Vec<ItemId>,
	modified: Vec<ItemId>,
}


/// Items come from model providers in all sorts of weird and wonderful ways.
/// The first step is to translate these forms into a unified type,
/// for integrating into our own stateful representations.
/// The [`key`] and [`status`] fields are particularly useful for diffing
/// state.
///
///
/// openresponses ----> PartialItem ----> Item
///
/// ## Duplicate Events
///
/// This type will reduce many matching events into an identical representation,
/// for example OutputTextDone and OutputItemDone, which is intended
/// as an equality check is used before reifying into an [`Item`].
///
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartialItem {
	pub key: PartialItemKey,
	pub status: ItemStatus,
	pub content: PartialContent,
}

impl PartialItem {
	pub fn from_output_items(
		items: impl IntoIterator<Item = OutputItem>,
		status: ItemStatus,
	) -> impl IntoIterator<Item = PartialItem> {
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
	/// defaulting to [`ItemStatus::Completed`]
	pub fn from_output_item(item: OutputItem, status: ItemStatus) -> Vec<Self> {
		match item {
			OutputItem::Message(message) => {
				let status = match message.status {
					MessageStatus::InProgress => ItemStatus::InProgress,
					MessageStatus::Completed => ItemStatus::Completed,
					MessageStatus::Incomplete => ItemStatus::Interrupted,
				};
				message
					.content
					.into_iter()
					.enumerate()
					.map(|(content_index, content)| {
						let key = PartialItemKey::Content {
							responses_id: message.id.clone(),
							content_index: content_index as u32,
						};
						PartialItem {
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
							ItemStatus::InProgress
						}
						FunctionCallStatus::Completed => ItemStatus::Completed,
						FunctionCallStatus::Incomplete => {
							ItemStatus::Interrupted
						}
					})
					.unwrap_or(status);
				vec![PartialItem {
					key: PartialItemKey::Single {
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
					FunctionCallStatus::InProgress => ItemStatus::InProgress,
					FunctionCallStatus::Completed => ItemStatus::Completed,
					FunctionCallStatus::Incomplete => ItemStatus::Interrupted,
				};
				vec![PartialItem {
					key: PartialItemKey::Single {
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
					out.push(PartialItem {
						key: PartialItemKey::Content {
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
					out.push(PartialItem {
						key: PartialItemKey::ReasoningSummary {
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
			key: PartialItemKey::Content {
				responses_id,
				content_index,
			},
			status: ItemStatus::InProgress,
			content: PartialContent::Delta(delta),
		}
	}
}


#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum PartialItemKey {
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
		// defaults to 0 when ommited
		// by streaming
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
	/// the function call must already exist and its arguments
	/// will be overwritten with this string.
	FunctionCallArgumentsDone(String),
}


impl PartialContent {
	pub fn into_content(self) -> Result<Content> {
		match self {
			PartialContent::OutputContent(output_content) => todo!(),
			PartialContent::ContentPart(content_part) => todo!(),
			PartialContent::FunctionCall {
				name,
				call_id,
				arguments,
			} => todo!(),
			PartialContent::FunctionCallOutput { call_id, output } => todo!(),
			PartialContent::ReasoningContent(_) => todo!(),
			PartialContent::ReasoningSummary(_) => todo!(),
			PartialContent::Delta(_) => {
				bevybail!("Cannot create content from a delta")
			}
			PartialContent::TextDone { text, logprobs } => todo!(),
			PartialContent::RefusalDone { refusal } => todo!(),
			PartialContent::ReasoningDone { content } => todo!(),
			PartialContent::AnnotationAdded {
				annotation_index,
				annotation,
			} => todo!(),
			PartialContent::FunctionCallArgumentsDone(_) => todo!(),
		}
		.xok()
	}

	pub fn apply(self, content: &mut Content) -> Result {
		match (self, content) {
			(
				PartialContent::OutputContent(output_content),
				Content::Text(text_item),
			) => todo!(),
			(
				PartialContent::OutputContent(output_content),
				Content::Refusal(refusal_item),
			) => todo!(),
			(
				PartialContent::OutputContent(output_content),
				Content::ReasoningSummary(reasoning_summary_item),
			) => todo!(),
			(
				PartialContent::OutputContent(output_content),
				Content::ReasoningContent(reasoning_content_item),
			) => todo!(),
			(
				PartialContent::OutputContent(output_content),
				Content::Url(url_item),
			) => todo!(),
			(
				PartialContent::OutputContent(output_content),
				Content::Bytes(bytes_item),
			) => todo!(),
			(
				PartialContent::OutputContent(output_content),
				Content::FunctionCall(function_call_item),
			) => todo!(),
			(
				PartialContent::OutputContent(output_content),
				Content::FunctionCallOutput(function_call_output_item),
			) => todo!(),
			(
				PartialContent::ContentPart(content_part),
				Content::Text(text_item),
			) => todo!(),
			(
				PartialContent::ContentPart(content_part),
				Content::Refusal(refusal_item),
			) => todo!(),
			(
				PartialContent::ContentPart(content_part),
				Content::ReasoningSummary(reasoning_summary_item),
			) => todo!(),
			(
				PartialContent::ContentPart(content_part),
				Content::ReasoningContent(reasoning_content_item),
			) => todo!(),
			(
				PartialContent::ContentPart(content_part),
				Content::Url(url_item),
			) => todo!(),
			(
				PartialContent::ContentPart(content_part),
				Content::Bytes(bytes_item),
			) => todo!(),
			(
				PartialContent::ContentPart(content_part),
				Content::FunctionCall(function_call_item),
			) => todo!(),
			(
				PartialContent::ContentPart(content_part),
				Content::FunctionCallOutput(function_call_output_item),
			) => todo!(),
			(
				PartialContent::FunctionCall {
					name,
					call_id,
					arguments,
				},
				Content::Text(text_item),
			) => todo!(),
			(
				PartialContent::FunctionCall {
					name,
					call_id,
					arguments,
				},
				Content::Refusal(refusal_item),
			) => todo!(),
			(
				PartialContent::FunctionCall {
					name,
					call_id,
					arguments,
				},
				Content::ReasoningSummary(reasoning_summary_item),
			) => todo!(),
			(
				PartialContent::FunctionCall {
					name,
					call_id,
					arguments,
				},
				Content::ReasoningContent(reasoning_content_item),
			) => todo!(),
			(
				PartialContent::FunctionCall {
					name,
					call_id,
					arguments,
				},
				Content::Url(url_item),
			) => todo!(),
			(
				PartialContent::FunctionCall {
					name,
					call_id,
					arguments,
				},
				Content::Bytes(bytes_item),
			) => todo!(),
			(
				PartialContent::FunctionCall {
					name,
					call_id,
					arguments,
				},
				Content::FunctionCall(function_call_item),
			) => todo!(),
			(
				PartialContent::FunctionCall {
					name,
					call_id,
					arguments,
				},
				Content::FunctionCallOutput(function_call_output_item),
			) => todo!(),
			(
				PartialContent::FunctionCallOutput { call_id, output },
				Content::Text(text_item),
			) => todo!(),
			(
				PartialContent::FunctionCallOutput { call_id, output },
				Content::Refusal(refusal_item),
			) => todo!(),
			(
				PartialContent::FunctionCallOutput { call_id, output },
				Content::ReasoningSummary(reasoning_summary_item),
			) => todo!(),
			(
				PartialContent::FunctionCallOutput { call_id, output },
				Content::ReasoningContent(reasoning_content_item),
			) => todo!(),
			(
				PartialContent::FunctionCallOutput { call_id, output },
				Content::Url(url_item),
			) => todo!(),
			(
				PartialContent::FunctionCallOutput { call_id, output },
				Content::Bytes(bytes_item),
			) => todo!(),
			(
				PartialContent::FunctionCallOutput { call_id, output },
				Content::FunctionCall(function_call_item),
			) => todo!(),
			(
				PartialContent::FunctionCallOutput { call_id, output },
				Content::FunctionCallOutput(function_call_output_item),
			) => todo!(),
			(PartialContent::ReasoningContent(_), Content::Text(text_item)) => {
				todo!()
			}
			(
				PartialContent::ReasoningContent(_),
				Content::Refusal(refusal_item),
			) => todo!(),
			(
				PartialContent::ReasoningContent(_),
				Content::ReasoningSummary(reasoning_summary_item),
			) => todo!(),
			(
				PartialContent::ReasoningContent(_),
				Content::ReasoningContent(reasoning_content_item),
			) => todo!(),
			(PartialContent::ReasoningContent(_), Content::Url(url_item)) => {
				todo!()
			}
			(
				PartialContent::ReasoningContent(_),
				Content::Bytes(bytes_item),
			) => todo!(),
			(
				PartialContent::ReasoningContent(_),
				Content::FunctionCall(function_call_item),
			) => todo!(),
			(
				PartialContent::ReasoningContent(_),
				Content::FunctionCallOutput(function_call_output_item),
			) => todo!(),
			(PartialContent::ReasoningSummary(_), Content::Text(text_item)) => {
				todo!()
			}
			(
				PartialContent::ReasoningSummary(_),
				Content::Refusal(refusal_item),
			) => todo!(),
			(
				PartialContent::ReasoningSummary(_),
				Content::ReasoningSummary(reasoning_summary_item),
			) => todo!(),
			(
				PartialContent::ReasoningSummary(_),
				Content::ReasoningContent(reasoning_content_item),
			) => todo!(),
			(PartialContent::ReasoningSummary(_), Content::Url(url_item)) => {
				todo!()
			}
			(
				PartialContent::ReasoningSummary(_),
				Content::Bytes(bytes_item),
			) => todo!(),
			(
				PartialContent::ReasoningSummary(_),
				Content::FunctionCall(function_call_item),
			) => todo!(),
			(
				PartialContent::ReasoningSummary(_),
				Content::FunctionCallOutput(function_call_output_item),
			) => todo!(),
			(PartialContent::Delta(_), Content::Text(text_item)) => todo!(),
			(PartialContent::Delta(_), Content::Refusal(refusal_item)) => {
				todo!()
			}
			(
				PartialContent::Delta(_),
				Content::ReasoningSummary(reasoning_summary_item),
			) => todo!(),
			(
				PartialContent::Delta(_),
				Content::ReasoningContent(reasoning_content_item),
			) => todo!(),
			(PartialContent::Delta(_), Content::Url(url_item)) => todo!(),
			(PartialContent::Delta(_), Content::Bytes(bytes_item)) => todo!(),
			(
				PartialContent::Delta(_),
				Content::FunctionCall(function_call_item),
			) => todo!(),
			(
				PartialContent::Delta(_),
				Content::FunctionCallOutput(function_call_output_item),
			) => todo!(),
			(
				PartialContent::TextDone { text, logprobs },
				Content::Text(text_item),
			) => todo!(),
			(
				PartialContent::TextDone { text, logprobs },
				Content::Refusal(refusal_item),
			) => todo!(),
			(
				PartialContent::TextDone { text, logprobs },
				Content::ReasoningSummary(reasoning_summary_item),
			) => todo!(),
			(
				PartialContent::TextDone { text, logprobs },
				Content::ReasoningContent(reasoning_content_item),
			) => todo!(),
			(
				PartialContent::TextDone { text, logprobs },
				Content::Url(url_item),
			) => todo!(),
			(
				PartialContent::TextDone { text, logprobs },
				Content::Bytes(bytes_item),
			) => todo!(),
			(
				PartialContent::TextDone { text, logprobs },
				Content::FunctionCall(function_call_item),
			) => todo!(),
			(
				PartialContent::TextDone { text, logprobs },
				Content::FunctionCallOutput(function_call_output_item),
			) => todo!(),
			(
				PartialContent::RefusalDone { refusal },
				Content::Text(text_item),
			) => todo!(),
			(
				PartialContent::RefusalDone { refusal },
				Content::Refusal(refusal_item),
			) => todo!(),
			(
				PartialContent::RefusalDone { refusal },
				Content::ReasoningSummary(reasoning_summary_item),
			) => todo!(),
			(
				PartialContent::RefusalDone { refusal },
				Content::ReasoningContent(reasoning_content_item),
			) => todo!(),
			(
				PartialContent::RefusalDone { refusal },
				Content::Url(url_item),
			) => todo!(),
			(
				PartialContent::RefusalDone { refusal },
				Content::Bytes(bytes_item),
			) => todo!(),
			(
				PartialContent::RefusalDone { refusal },
				Content::FunctionCall(function_call_item),
			) => todo!(),
			(
				PartialContent::RefusalDone { refusal },
				Content::FunctionCallOutput(function_call_output_item),
			) => todo!(),
			(
				PartialContent::ReasoningDone { content },
				Content::Text(text_item),
			) => todo!(),
			(
				PartialContent::ReasoningDone { content },
				Content::Refusal(refusal_item),
			) => todo!(),
			(
				PartialContent::ReasoningDone { content },
				Content::ReasoningSummary(reasoning_summary_item),
			) => todo!(),
			(
				PartialContent::ReasoningDone { content },
				Content::ReasoningContent(reasoning_content_item),
			) => todo!(),
			(
				PartialContent::ReasoningDone { content },
				Content::Url(url_item),
			) => todo!(),
			(
				PartialContent::ReasoningDone { content },
				Content::Bytes(bytes_item),
			) => todo!(),
			(
				PartialContent::ReasoningDone { content },
				Content::FunctionCall(function_call_item),
			) => todo!(),
			(
				PartialContent::ReasoningDone { content },
				Content::FunctionCallOutput(function_call_output_item),
			) => todo!(),
			(
				PartialContent::AnnotationAdded {
					annotation_index,
					annotation,
				},
				Content::Text(text_item),
			) => todo!(),
			(
				PartialContent::AnnotationAdded {
					annotation_index,
					annotation,
				},
				Content::Refusal(refusal_item),
			) => todo!(),
			(
				PartialContent::AnnotationAdded {
					annotation_index,
					annotation,
				},
				Content::ReasoningSummary(reasoning_summary_item),
			) => todo!(),
			(
				PartialContent::AnnotationAdded {
					annotation_index,
					annotation,
				},
				Content::ReasoningContent(reasoning_content_item),
			) => todo!(),
			(
				PartialContent::AnnotationAdded {
					annotation_index,
					annotation,
				},
				Content::Url(url_item),
			) => todo!(),
			(
				PartialContent::AnnotationAdded {
					annotation_index,
					annotation,
				},
				Content::Bytes(bytes_item),
			) => todo!(),
			(
				PartialContent::AnnotationAdded {
					annotation_index,
					annotation,
				},
				Content::FunctionCall(function_call_item),
			) => todo!(),
			(
				PartialContent::AnnotationAdded {
					annotation_index,
					annotation,
				},
				Content::FunctionCallOutput(function_call_output_item),
			) => todo!(),
			(
				PartialContent::FunctionCallArgumentsDone(_),
				Content::Text(text_item),
			) => todo!(),
			(
				PartialContent::FunctionCallArgumentsDone(_),
				Content::Refusal(refusal_item),
			) => todo!(),
			(
				PartialContent::FunctionCallArgumentsDone(_),
				Content::ReasoningSummary(reasoning_summary_item),
			) => todo!(),
			(
				PartialContent::FunctionCallArgumentsDone(_),
				Content::ReasoningContent(reasoning_content_item),
			) => todo!(),
			(
				PartialContent::FunctionCallArgumentsDone(_),
				Content::Url(url_item),
			) => todo!(),
			(
				PartialContent::FunctionCallArgumentsDone(_),
				Content::Bytes(bytes_item),
			) => todo!(),
			(
				PartialContent::FunctionCallArgumentsDone(_),
				Content::FunctionCall(function_call_item),
			) => todo!(),
			(
				PartialContent::FunctionCallArgumentsDone(_),
				Content::FunctionCallOutput(function_call_output_item),
			) => todo!(),
		}
		Ok(())
	}
}
