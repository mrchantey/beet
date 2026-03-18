use crate::openresponses::Annotation;
use crate::openresponses::ContentPart;
use crate::openresponses::FunctionCallStatus;
use crate::openresponses::LogProb;
use crate::openresponses::MessageStatus;
use crate::openresponses::OutputContent;
use crate::openresponses::OutputItem;
use crate::openresponses::request::FunctionCallOutputParam;
use crate::openresponses::request::FunctionCallParam;
use crate::openresponses::request::FunctionOutputContent;
use crate::openresponses::request::Input;
use crate::openresponses::request::InputItem;
use crate::openresponses::request::MessageContent;
use crate::openresponses::request::MessageParam;
use crate::prelude::*;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;


#[derive(Debug, Default)]
pub struct PartialItemMap {
	annotation_inliner: AnnotationInliner,
	/// Map an openresponses call id to an [`ItemId`]
	call_id_to_item_id: HashMap<String, ItemId>,
	item_id_to_call_id: HashMap<ItemId, String>,
	/// Map partial item keys to the created [`ItemId`]
	item_key_map: HashMap<PartialItemKey, ItemId>,
}

impl PartialItemMap {
	pub fn new() -> Self { Self::default() }

	pub fn with_annotation_inliner(
		mut self,
		inliner: AnnotationInliner,
	) -> Self {
		self.annotation_inliner = inliner;
		self
	}

	// ═══════════════════════════════════════════════════════════════════════
	// Call ID mapping
	// ═══════════════════════════════════════════════════════════════════════

	fn set_call_id(&mut self, item_id: ItemId, call_id: String) -> Result {
		if self.item_id_to_call_id.contains_key(&item_id) {
			bevybail!("item_id {item_id} already has a call_id");
		} else if self.call_id_to_item_id.contains_key(&call_id) {
			bevybail!("call_id {call_id} already has an item_id");
		}
		self.call_id_to_item_id.insert(call_id.clone(), item_id);
		self.item_id_to_call_id.insert(item_id, call_id);
		Ok(())
	}

	fn get_call_id(&self, item_id: ItemId) -> Result<String> {
		self.item_id_to_call_id
			.get(&item_id)
			.cloned()
			.ok_or_else(|| {
				bevyhow!("no call_id registered for item_id {item_id}")
			})
	}

	fn get_item_id_for_call(&self, call_id: &str) -> Result<ItemId> {
		self.call_id_to_item_id
			.get(call_id)
			.cloned()
			.ok_or_else(|| {
				bevyhow!("no item_id registered for call_id {call_id}")
			})
	}

	// ═══════════════════════════════════════════════════════════════════════
	// Response item key mapping
	// ═══════════════════════════════════════════════════════════════════════

	fn set_response_item(
		&mut self,
		key: PartialItemKey,
		item_id: ItemId,
	) -> Result {
		if self.item_key_map.contains_key(&key) {
			bevybail!("item_key_map already has an entry for key {key:?}");
		} else {
			self.item_key_map.insert(key, item_id);
		}
		Ok(())
	}

	pub fn get_response_item(&self, key: &PartialItemKey) -> Result<ItemId> {
		self.item_key_map.get(key).cloned().ok_or_else(|| {
			bevyhow!("no item_id registered for responses item key {key:?}")
		})
	}

	// ═══════════════════════════════════════════════════════════════════════
	// Input building - converts our items to openresponses input format
	// ═══════════════════════════════════════════════════════════════════════

	pub fn build_input(
		&self,
		map: &ContextMap,
		agent_id: ActorId,
		thread_id: ThreadId,
		last_sent_item: Option<ItemId>,
	) -> Result<openresponses::request::Input> {
		let thread = map.threads().get(thread_id)?;

		let items = if let Some(last_sent_item) = last_sent_item {
			thread.items_after(last_sent_item)
		} else {
			thread.items()
		};

		// threads are strictly already chronologically sorted by uuidv7
		let items = items.into_iter().xtry_map(|item_id| {
			self.into_openresponses_input(map, agent_id, *item_id)
		})?;

		Input::Items(items).xok()
	}

	/// Map an item to an openresponses input, relative to a given actor.
	/// The provided actor is used to correctly assign [`MessageRole::Assistant`]
	/// for 'self' messages, and [`MessageRole::User`] for all others.
	fn into_openresponses_input(
		&self,
		map: &ContextMap,
		agent_id: ActorId,
		item_id: ItemId,
	) -> Result<openresponses::request::InputItem> {
		let item = map.items().get(item_id)?;
		let owner = map.actors().get(item.owner())?;
		let role = item_message_role(agent_id, owner);

		let input_item = match item.content() {
			Content::Text(TextItem(value)) => {
				InputItem::Message(MessageParam {
					id: None,
					role,
					content: MessageContent::Text(value.clone()),
					status: None,
				})
			}
			Content::Refusal(RefusalItem(value)) => {
				InputItem::Message(MessageParam {
					id: None,
					role,
					content: MessageContent::Text(value.clone()),
					status: None,
				})
			}
			Content::ReasoningSummary(ReasoningSummaryItem(value)) => {
				InputItem::Message(MessageParam {
					id: None,
					role,
					content: MessageContent::Text(value.clone()),
					status: None,
				})
			}
			Content::ReasoningContent(ReasoningContentItem(value)) => {
				InputItem::Message(MessageParam {
					id: None,
					role,
					content: MessageContent::Text(value.clone()),
					status: None,
				})
			}
			Content::Url(url_item) => InputItem::Message(MessageParam {
				id: None,
				role,
				content: MessageContent::Parts(vec![ContentPart::InputFile(
					openresponses::InputFile {
						filename: Some(url_item.filename()),
						file_data: None,
						file_url: Some(url_item.url().to_string()),
					},
				)]),
				status: None,
			}),
			Content::Bytes(bytes_item) => InputItem::Message(MessageParam {
				id: None,
				role,
				content: MessageContent::Parts(vec![ContentPart::InputFile(
					openresponses::InputFile {
						filename: Some(bytes_item.filename()),
						file_data: Some(bytes_item.bytes_base64()),
						file_url: None,
					},
				)]),
				status: None,
			}),
			Content::FunctionCall(function_call) => {
				InputItem::FunctionCall(FunctionCallParam {
					id: None,
					call_id: self.get_call_id(item.id())?,
					name: function_call.function_name().to_string(),
					arguments: function_call.args().to_string(),
					status: None,
				})
			}
			Content::FunctionCallOutput(output_item) => {
				InputItem::FunctionCallOutput(FunctionCallOutputParam {
					id: None,
					call_id: self
						.get_call_id(output_item.function_call_item)?,
					output: FunctionOutputContent::Text(
						output_item.output().to_string(),
					),
					status: None,
				})
			}
		};
		input_item.xok()
	}

	// ═══════════════════════════════════════════════════════════════════════
	// Output parsing - converts partial items into our item format
	// ═══════════════════════════════════════════════════════════════════════

	/// Apply a stream of partial items against the item map, producing
	/// [`ItemChanges`] describing what was created or modified.
	/// Also registers call_id and response_item mappings for new items.
	pub fn apply_items(
		&mut self,
		item_map: &mut DocMap<Item>,
		owner: ActorId,
		partial_items: impl IntoIterator<Item = PartialItem>,
	) -> Result<ItemChanges> {
		let mut changes = ItemChanges::default();
		for partial_item in partial_items {
			if let Some(&item_id) = self.item_key_map.get(&partial_item.key) {
				// item already exists, update it in place
				let item = item_map.get_mut(item_id)?;
				let before_mutation = item.hash();

				item.set_status(partial_item.status);
				self.apply_partial_content(
					partial_item.content.clone(),
					item.content_mut(),
				)?;
				let after_mutation = item.hash();
				if before_mutation != after_mutation {
					// it changed!
					changes.modified.push(item_id);
					// discard already registered error on updates
					let _ = self.register_call_id(&partial_item, item_id);
				}
			} else {
				// create new item
				let content = self.partial_content_into_content(
					&partial_item.key,
					partial_item.content.clone(),
				)?;
				let item = Item::new(owner, partial_item.status, content);
				let item_id = item.id();

				// register response key -> item id
				self.set_response_item(partial_item.key.clone(), item_id)?;

				// register call_id mappings for function calls
				self.register_call_id(&partial_item, item_id)?;

				changes.created.push(item_id);
				item_map.insert(item);
			}
		}
		changes.xok()
	}

	// try to extract and register a call_id from a partial item
	fn register_call_id(
		&mut self,
		partial: &PartialItem,
		item_id: ItemId,
	) -> Result {
		match &partial.content {
			PartialContent::FunctionCall { call_id, .. } => {
				self.set_call_id(item_id, call_id.clone())?;
			}
			_ => {}
		}
		Ok(())
	}

	// ═══════════════════════════════════════════════════════════════════════
	// Content conversion helpers using the annotation inliner
	// ═══════════════════════════════════════════════════════════════════════

	fn inline_output_text_annotations(
		&self,
		text: &str,
		annotations: &[Annotation],
	) -> String {
		self.annotation_inliner
			.inline_annotations(text, annotations)
	}

	/// Convert a [`PartialContent`] into a finalized [`Content`], inlining
	/// annotations as markdown where appropriate.
	/// The key is used to disambiguate content types for generic variants
	/// like [`PartialContent::Delta`].
	fn partial_content_into_content(
		&self,
		key: &PartialItemKey,
		partial: PartialContent,
	) -> Result<Content> {
		match partial {
			PartialContent::OutputContent(OutputContent::OutputText(t)) => {
				let text = if t.annotations.is_empty() {
					t.text
				} else {
					self.inline_output_text_annotations(
						&t.text,
						&t.annotations,
					)
				};
				Content::Text(TextItem(text))
			}
			PartialContent::OutputContent(OutputContent::Refusal(r)) => {
				Content::Refusal(RefusalItem(r.refusal))
			}
			PartialContent::ContentPart(ContentPart::InputText(t)) => {
				Content::Text(TextItem(t.text))
			}
			PartialContent::ContentPart(ContentPart::OutputText(t)) => {
				let text = if t.annotations.is_empty() {
					t.text
				} else {
					self.inline_output_text_annotations(
						&t.text,
						&t.annotations,
					)
				};
				Content::Text(TextItem(text))
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
			PartialContent::FunctionCall { name, arguments, call_id: _ } => {
				Content::FunctionCall(FunctionCallItem { name, arguments })
			}
			PartialContent::FunctionCallOutput { call_id, output } => {
				// resolve call_id to item_id
				let function_call_item =
					self.get_item_id_for_call(&call_id)?;
				Content::FunctionCallOutput(FunctionCallOutputItem {
					function_call_item,
					output,
				})
			}
			PartialContent::ReasoningContent(text) => {
				Content::ReasoningContent(ReasoningContentItem(text))
			}
			PartialContent::ReasoningSummary(text) => {
				Content::ReasoningSummary(ReasoningSummaryItem(text))
			}
			PartialContent::Delta(delta) => {
				// during streaming, a delta may arrive before the item
				// is created (eg OutputTextDelta before OutputItemAdded
				// has content). Use the key to determine the content type.
				match key {
					PartialItemKey::ReasoningSummary { .. } => {
						Content::ReasoningSummary(ReasoningSummaryItem(delta))
					}
					PartialItemKey::Single { .. } => {
						// single-key items are function calls
						Content::FunctionCall(FunctionCallItem {
							name: String::new(),
							arguments: delta,
						})
					}
					PartialItemKey::Content { .. } => {
						// default to text for content-keyed deltas
						Content::Text(TextItem(delta))
					}
				}
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
				bevybail!("Cannot create content from an annotation event alone")
			}
			PartialContent::FunctionCallArgumentsDone(_) => {
				bevybail!("Cannot create content from FunctionCallArgumentsDone without name and call_id context")
			}
		}
		.xok()
	}

	/// Apply a [`PartialContent`] to an existing [`Content`], mutating
	/// it in place. Handles deltas, replacements, and annotation inlining.
	fn apply_partial_content(
		&self,
		partial: PartialContent,
		content: &mut Content,
	) -> Result {
		match (partial, content) {
			// OutputContent replaces matching content wholesale
			(
				PartialContent::OutputContent(OutputContent::OutputText(t)),
				Content::Text(item),
			) => {
				item.0 = if t.annotations.is_empty() {
					t.text
				} else {
					self.inline_output_text_annotations(&t.text, &t.annotations)
				};
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
				item.0 = if t.annotations.is_empty() {
					t.text
				} else {
					self.inline_output_text_annotations(&t.text, &t.annotations)
				};
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
					call_id: _,
				},
				Content::FunctionCall(item),
			) => {
				item.name = name;
				item.arguments = arguments;
			}
			// FunctionCallOutput updates the output string and resolves call_id
			(
				PartialContent::FunctionCallOutput { output, call_id },
				Content::FunctionCallOutput(item),
			) => {
				item.output = output;
				// update the function_call_item reference if possible
				if let Ok(fc_item_id) = self.get_item_id_for_call(&call_id) {
					item.function_call_item = fc_item_id;
				}
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
			// AnnotationAdded: inline annotation into existing text
			(
				PartialContent::AnnotationAdded { annotation, .. },
				Content::Text(item),
			) => {
				item.0 =
					self.inline_output_text_annotations(&item.0, &[annotation]);
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

#[derive(Default)]
pub struct ItemChanges {
	pub created: Vec<ItemId>,
	pub modified: Vec<ItemId>,
}

impl ItemChanges {
	pub fn is_empty(&self) -> bool {
		self.created.is_empty() && self.modified.is_empty()
	}

	/// All item ids that were either created or modified
	pub fn all_items(&self) -> impl Iterator<Item = &ItemId> {
		self.created.iter().chain(self.modified.iter())
	}
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


/// Get the message role for this actor, relative to the items actor id.
/// This is useful when an agent is constructing its context for an
/// openresponses request.
fn item_message_role(
	agent_id: ActorId,
	owner: &Actor,
) -> openresponses::MessageRole {
	use openresponses::MessageRole;
	match owner.kind() {
		ActorKind::System => MessageRole::System,
		ActorKind::Developer => MessageRole::Developer,
		ActorKind::Human => MessageRole::User,
		ActorKind::Agent => {
			if owner.id() == agent_id {
				MessageRole::Assistant
			} else {
				MessageRole::User
			}
		}
	}
}
