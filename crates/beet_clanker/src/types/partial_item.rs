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
	) -> Result<ItemChanges> {
		let mut outcomes = ItemChanges::default();
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
pub struct ItemChanges {
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
			PartialContent::OutputContent(OutputContent::OutputText(t)) => {
				Content::Text(TextItem(t.text))
			}
			PartialContent::OutputContent(OutputContent::Refusal(r)) => {
				Content::Refusal(RefusalItem(r.refusal))
			}
			PartialContent::ContentPart(ContentPart::InputText(t)) => {
				Content::Text(TextItem(t.text))
			}
			PartialContent::ContentPart(ContentPart::OutputText(t)) => {
				if !t.annotations.is_empty(){
					todo!("inline annotations as markdown")
				}
				Content::Text(TextItem(t.text))
			}
			PartialContent::ContentPart(ContentPart::Refusal(r)) => {
				Content::Refusal(RefusalItem(r.refusal))
			}
			PartialContent::ContentPart(ContentPart::ReasoningText(r)) => {
				Content::ReasoningContent(ReasoningContentItem(r.text))
			}
			PartialContent::ContentPart(ContentPart::SummaryText(s)) => {
				Content::ReasoningSummary(ReasoningSummaryItem(s.text))
			}
			PartialContent::ContentPart(ContentPart::InputImage(img)) => {
				Content::Url(UrlItem {
					file_stem: None,
					media_type: MediaType::from_path(&img.image_url),
					url: img.image_url.into(),
				})
			}
			PartialContent::ContentPart(ContentPart::InputVideo(vid)) => {
				Content::Url(UrlItem {
					file_stem: None,
					// this is an assumption?
					media_type: MediaType::Mp4,
					url: vid.video_url.into(),
				})
			}
			PartialContent::ContentPart(ContentPart::InputFile(file)) => {
				let media_type = file
					.filename
					.as_deref()
					.map(MediaType::from_path)
					.unwrap_or(MediaType::Bytes);
				let file_stem = file.filename.as_deref().and_then(|f| {
					std::path::Path::new(f)
						.file_stem()
						.and_then(|s| s.to_str())
						.map(|s| s.to_string())
				});
				if let Some(url) = file.file_url {
					Content::Url(UrlItem {
						file_stem,
						media_type,
						url: url.into(),
					})
				} else if let Some(data) = file.file_data {
					use base64::Engine;
					let bytes = base64::prelude::BASE64_STANDARD
						.decode(&data)
						.map_err(|err| bevyhow!("Failed to decode base64 file data: {err}"))?;
					Content::Bytes(BytesItem {
						file_stem,
						media_type,
						bytes,
					})
				} else {
					bevybail!("InputFile has neither file_url nor file_data")
				}
			}
			PartialContent::FunctionCall { name, arguments, call_id:_ } => {
				// we represent call id as the function call item id
				Content::FunctionCall(FunctionCallItem { name, arguments })
			}
			PartialContent::FunctionCallOutput { .. } => {
				bevybail!("Cannot create FunctionCallOutput content without item id context")
			}
			PartialContent::ReasoningContent(text) => {
				Content::ReasoningContent(ReasoningContentItem(text))
			}
			PartialContent::ReasoningSummary(text) => {
				Content::ReasoningSummary(ReasoningSummaryItem(text))
			}
			PartialContent::Delta(_) => {
				bevybail!("Cannot create content from a delta")
			}
			PartialContent::TextDone { text, .. } => {
				Content::Text(TextItem(text))
			}
			PartialContent::RefusalDone { refusal } => {
				Content::Refusal(RefusalItem(refusal))
			}
			PartialContent::ReasoningDone { content } => {
				Content::ReasoningContent(ReasoningContentItem(content))
			}
			PartialContent::AnnotationAdded { .. } => {
				bevybail!("Cannot create content from an annotation event")
			}
			PartialContent::FunctionCallArgumentsDone(_) => {
				bevybail!("Cannot create content from FunctionCallArgumentsDone without name and call_id context")
			}
		}
		.xok()
	}

	pub fn apply(self, content: &mut Content) -> Result {
		match (self, content) {
			// OutputContent replaces matching content wholesale
			(
				PartialContent::OutputContent(OutputContent::OutputText(t)),
				Content::Text(item),
			) => {
				if !t.annotations.is_empty() {
					todo!("inline annotations as markdown")
				}
				item.0 = t.text;
			}
			(
				PartialContent::OutputContent(OutputContent::Refusal(r)),
				Content::Refusal(item),
			) => {
				item.0 = r.refusal;
			}
			// ContentPart replaces matching content wholesale
			(
				PartialContent::ContentPart(ContentPart::InputText(t)),
				Content::Text(item),
			) => {
				item.0 = t.text;
			}
			(
				PartialContent::ContentPart(ContentPart::OutputText(t)),
				Content::Text(item),
			) => {
				if !t.annotations.is_empty() {
					todo!("inline annotations as markdown")
				}
				item.0 = t.text;
			}
			(
				PartialContent::ContentPart(ContentPart::Refusal(r)),
				Content::Refusal(item),
			) => {
				item.0 = r.refusal;
			}
			(
				PartialContent::ContentPart(ContentPart::ReasoningText(r)),
				Content::ReasoningContent(item),
			) => {
				item.0 = r.text;
			}
			(
				PartialContent::ContentPart(ContentPart::SummaryText(s)),
				Content::ReasoningSummary(item),
			) => {
				item.0 = s.text;
			}
			// FunctionCall updates name and arguments
			(
				PartialContent::FunctionCall {
					name,
					arguments,
					// we represent call id as the function call item id
					call_id: _,
				},
				Content::FunctionCall(item),
			) => {
				item.name = name;
				item.arguments = arguments;
			}
			// FunctionCallOutput updates the output string
			(
				PartialContent::FunctionCallOutput { output, call_id },
				Content::FunctionCallOutput(item),
			) => {
				item.output = output;
				todo!(
					"resolve call id,even if already resolved we should overwrite"
				);
			}
			// ReasoningContent/Summary replace in place
			(
				PartialContent::ReasoningContent(text),
				Content::ReasoningContent(item),
			) => {
				item.0 = text;
			}
			(
				PartialContent::ReasoningSummary(text),
				Content::ReasoningSummary(item),
			) => {
				item.0 = text;
			}
			// Deltas append to the appropriate content type
			(PartialContent::Delta(delta), Content::Text(item)) => {
				item.0.push_str(&delta);
			}
			(PartialContent::Delta(delta), Content::Refusal(item)) => {
				item.0.push_str(&delta);
			}
			(PartialContent::Delta(delta), Content::ReasoningContent(item)) => {
				item.0.push_str(&delta);
			}
			(PartialContent::Delta(delta), Content::ReasoningSummary(item)) => {
				item.0.push_str(&delta);
			}
			(PartialContent::Delta(delta), Content::FunctionCall(item)) => {
				item.arguments.push_str(&delta);
			}
			// TextDone overwrites text content
			(PartialContent::TextDone { text, .. }, Content::Text(item)) => {
				item.0 = text;
			}
			// RefusalDone overwrites refusal content
			(
				PartialContent::RefusalDone { refusal },
				Content::Refusal(item),
			) => {
				item.0 = refusal;
			}
			// ReasoningDone overwrites reasoning content
			(
				PartialContent::ReasoningDone { content },
				Content::ReasoningContent(item),
			) => {
				item.0 = content;
			}
			// AnnotationAdded: todo — inline as markdown link
			(PartialContent::AnnotationAdded { .. }, Content::Text(_)) => {
				// TODO inline annotation as markdown link
			}
			// FunctionCallArgumentsDone overwrites arguments
			(
				PartialContent::FunctionCallArgumentsDone(args),
				Content::FunctionCall(item),
			) => {
				item.arguments = args;
			}
			// Mismatched or unsupported combinations
			(partial, content) => {
				bevybail!(
					"Cannot apply {:?} to {:?}",
					std::mem::discriminant(&partial),
					content.kind()
				)
			}
		}
		Ok(())
	}
}
