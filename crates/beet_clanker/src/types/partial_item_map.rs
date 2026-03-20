use crate::openresponses::Annotation;
use crate::openresponses::ContentPart;
use crate::openresponses::MessageRole;
use crate::openresponses::OutputContent;
use crate::openresponses::request::FunctionCallOutputParam;
use crate::openresponses::request::FunctionCallParam;
use crate::openresponses::request::FunctionOutputContent;
use crate::openresponses::request::Input;
use crate::openresponses::request::InputItem;
use crate::openresponses::request::MessageContent;
use crate::openresponses::request::MessageParam;
use crate::prelude::*;
use beet_core::prelude::*;

#[derive(Debug, Default)]
pub struct PartialItemMap {
	annotation_inliner: AnnotationInliner,
	/// Map an openresponses call id to an [`ActionId`]
	call_id_to_action_id: HashMap<String, ActionId>,
	action_id_to_call_id: HashMap<ActionId, String>,
	/// Map partial item keys to the created [`ActionId`]
	action_key_map: HashMap<PartialItemKey, ActionId>,
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

	fn set_call_id(&mut self, action_id: ActionId, call_id: String) -> Result {
		if self.action_id_to_call_id.contains_key(&action_id) {
			bevybail!("action_id {action_id} already has a call_id");
		} else if self.call_id_to_action_id.contains_key(&call_id) {
			bevybail!("call_id {call_id} already has an action_id");
		}
		self.call_id_to_action_id.insert(call_id.clone(), action_id);
		self.action_id_to_call_id.insert(action_id, call_id);
		Ok(())
	}

	fn get_call_id(&self, action_id: ActionId) -> Result<String> {
		self.action_id_to_call_id
			.get(&action_id)
			.cloned()
			.ok_or_else(|| {
				bevyhow!("no call_id registered for action_id {action_id}")
			})
	}

	fn get_action_id_for_call(&self, call_id: &str) -> Result<ActionId> {
		self.call_id_to_action_id
			.get(call_id)
			.cloned()
			.ok_or_else(|| {
				bevyhow!("no action_id registered for call_id {call_id}")
			})
	}

	// ═══════════════════════════════════════════════════════════════════════
	// Response item key mapping
	// ═══════════════════════════════════════════════════════════════════════

	fn set_response_item(
		&mut self,
		key: PartialItemKey,
		action_id: ActionId,
	) -> Result {
		if self.action_key_map.contains_key(&key) {
			bevybail!("action_key_map already has an entry for key {key:?}");
		} else {
			self.action_key_map.insert(key, action_id);
		}
		Ok(())
	}

	pub fn get_response_item(&self, key: &PartialItemKey) -> Result<ActionId> {
		self.action_key_map.get(key).cloned().ok_or_else(|| {
			bevyhow!("no action_id registered for responses item key {key:?}")
		})
	}

	// ═══════════════════════════════════════════════════════════════════════
	// Input building - converts our actions to openresponses input format
	// ═══════════════════════════════════════════════════════════════════════

	pub fn build_o11s_input(
		&self,
		map: &ContextMap,
		agent_id: ActorId,
		thread_id: ThreadId,
		last_sent_action: Option<ActionId>,
	) -> Result<openresponses::request::Input> {
		// collect and sort action ids for this thread (uuid7 = chronological)
		let mut action_ids: Vec<ActionId> = map
			.actions()
			.values()
			.filter(|action| action.thread() == thread_id)
			.map(|action| action.id())
			.collect();
		action_ids.sort();

		let action_ids = if let Some(last_sent) = last_sent_action {
			match action_ids.binary_search(&last_sent) {
				Ok(i) => action_ids[i + 1..].to_vec(),
				Err(i) => action_ids[i..].to_vec(),
			}
		} else {
			action_ids
		};

		// actions are strictly already chronologically sorted by uuidv7
		let items = action_ids.into_iter().xtry_map(|action_id| {
			self.action_to_o11s_input(map, agent_id, action_id)
		})?;

		Input::Items(items).xok()
	}

	/// Map an action to an openresponses input, relative to a given actor.
	/// The provided actor is used to correctly assign [`MessageRole::Assistant`]
	/// for 'self' messages, and [`MessageRole::User`] for all others.
	fn action_to_o11s_input(
		&self,
		map: &ContextMap,
		agent_id: ActorId,
		action_id: ActionId,
	) -> Result<openresponses::request::InputItem> {
		let action = map.actions().get(action_id)?;
		let author = map.actors().get(action.author())?;
		let role = match author.kind() {
			ActorKind::System => MessageRole::System,
			ActorKind::App => MessageRole::Developer,
			ActorKind::Agent => {
				if author.id() == agent_id {
					MessageRole::Assistant
				} else {
					MessageRole::User
				}
			}
			ActorKind::User => MessageRole::User,
		};

		let input_item = match action.payload() {
			ActionPayload::Text(TextItem(value)) => {
				InputItem::Message(MessageParam {
					id: None,
					role,
					content: MessageContent::Text(value.clone()),
					status: None,
				})
			}
			ActionPayload::Refusal(RefusalItem(value)) => {
				InputItem::Message(MessageParam {
					id: None,
					role,
					content: MessageContent::Text(value.clone()),
					status: None,
				})
			}
			ActionPayload::ReasoningSummary(ReasoningSummaryItem(value)) => {
				InputItem::Message(MessageParam {
					id: None,
					role,
					content: MessageContent::Text(value.clone()),
					status: None,
				})
			}
			ActionPayload::ReasoningContent(ReasoningContentItem(value)) => {
				InputItem::Message(MessageParam {
					id: None,
					role,
					content: MessageContent::Text(value.clone()),
					status: None,
				})
			}
			ActionPayload::Url(url_item) => InputItem::Message(MessageParam {
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
			ActionPayload::Bytes(bytes_item) => {
				InputItem::Message(MessageParam {
					id: None,
					role,
					content: MessageContent::Parts(vec![
						ContentPart::InputFile(openresponses::InputFile {
							filename: Some(bytes_item.filename()),
							file_data: Some(bytes_item.bytes_base64()),
							file_url: None,
						}),
					]),
					status: None,
				})
			}
			ActionPayload::FunctionCall(function_call) => {
				InputItem::FunctionCall(FunctionCallParam {
					id: None,
					call_id: self.get_call_id(action.id())?,
					name: function_call.function_name().to_string(),
					arguments: function_call.args().to_string(),
					status: None,
				})
			}
			ActionPayload::FunctionCallOutput(output_item) => {
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
	// Output parsing - converts partial items into our action format
	// ═══════════════════════════════════════════════════════════════════════

	/// Apply a stream of partial items against the action map, producing
	/// [`ActionChanges`] describing what was created or modified.
	/// Also registers call_id and response_item mappings for new actions.
	pub fn apply_actions(
		&mut self,
		action_map: &mut DocMap<Action>,
		author: ActorId,
		thread: ThreadId,
		partial_items: impl IntoIterator<Item = PartialItem>,
	) -> Result<ActionChanges> {
		let mut changes = ActionChanges::default();
		for partial_item in partial_items {
			let PartialItem {
				key,
				status,
				content,
			} = partial_item;

			// extract call_id before content is moved
			let call_id = match &content {
				PartialContent::FunctionCall { call_id, .. } => {
					Some(call_id.clone())
				}
				_ => None,
			};

			if let Some(&action_id) = self.action_key_map.get(&key) {
				// action already exists, update it in place
				let action = action_map.get_mut(action_id)?;
				let before_mutation = action.hash();

				action.set_status(status);
				self.apply_partial_content(
					&key,
					content,
					action.payload_mut(),
				)?;
				let after_mutation = action.hash();
				if before_mutation != after_mutation {
					changes.modified.push(action_id);
					// discard already registered error on updates
					if let Some(call_id) = call_id {
						let _ = self.set_call_id(action_id, call_id);
					}
				}
			} else {
				// create new action
				let payload =
					self.partial_content_into_payload(&key, content)?;
				let action = Action::new(author, thread, status, payload);
				let action_id = action.id();

				// register response key -> action id
				self.set_response_item(key, action_id)?;

				// register call_id mapping for function calls
				if let Some(call_id) = call_id {
					self.set_call_id(action_id, call_id)?;
				}

				changes.created.push(action_id);
				action_map.insert(action);
			}
		}
		changes.xok()
	}

	// ═══════════════════════════════════════════════════════════════════════
	// Payload conversion helpers using the annotation inliner
	// ═══════════════════════════════════════════════════════════════════════

	fn inline_output_text_annotations(
		&self,
		text: &str,
		annotations: &[Annotation],
	) -> String {
		self.annotation_inliner
			.inline_annotations(text, annotations)
	}

	/// Convert a [`PartialContent`] into a finalized [`ActionPayload`], inlining
	/// annotations as markdown where appropriate.
	/// The key is used to disambiguate content types for generic variants
	/// like [`PartialContent::Delta`].
	fn partial_content_into_payload(
		&self,
		key: &PartialItemKey,
		partial: PartialContent,
	) -> Result<ActionPayload> {
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
				ActionPayload::Text(TextItem(text))
			}
			PartialContent::OutputContent(OutputContent::Refusal(r)) => {
				ActionPayload::Refusal(RefusalItem(r.refusal))
			}
			PartialContent::ContentPart(ContentPart::InputText(t)) => {
				ActionPayload::Text(TextItem(t.text))
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
				ActionPayload::Text(TextItem(text))
			}
			PartialContent::ContentPart(ContentPart::Refusal(r)) => {
				ActionPayload::Refusal(RefusalItem(r.refusal))
			}
			PartialContent::ContentPart(ContentPart::ReasoningText(r)) => {
				ActionPayload::ReasoningContent(ReasoningContentItem(r.text))
			}
			PartialContent::ContentPart(ContentPart::SummaryText(s)) => {
				ActionPayload::ReasoningSummary(ReasoningSummaryItem(s.text))
			}
			PartialContent::ContentPart(ContentPart::InputImage(img)) => {
				ActionPayload::Url(UrlItem {
					file_stem: None,
					media_type: MediaType::from_path(&img.image_url),
					url: img.image_url.into(),
				})
			}
			PartialContent::ContentPart(ContentPart::InputVideo(vid)) => {
				ActionPayload::Url(UrlItem {
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
					ActionPayload::Url(UrlItem {
						file_stem,
						media_type,
						url: url.into(),
					})
				} else if let Some(data) = file.file_data {
					use base64::Engine;
					let bytes = base64::prelude::BASE64_STANDARD
						.decode(&data)
						.map_err(|err| {
							bevyhow!(
								"Failed to decode base64 file data: {err}"
							)
						})?;
					ActionPayload::Bytes(BytesItem {
						file_stem,
						media_type,
						bytes,
					})
				} else {
					bevybail!("InputFile has neither file_url nor file_data")
				}
			}
			PartialContent::FunctionCall {
				name,
				arguments,
				call_id: _,
			} => ActionPayload::FunctionCall(FunctionCallItem {
				name,
				arguments,
			}),
			PartialContent::FunctionCallOutput { call_id, output } => {
				// resolve call_id to action_id
				let function_call_item =
					self.get_action_id_for_call(&call_id)?;
				ActionPayload::FunctionCallOutput(FunctionCallOutputItem {
					function_call_item,
					output,
				})
			}
			PartialContent::ReasoningContent(text) => {
				ActionPayload::ReasoningContent(ReasoningContentItem(text))
			}
			PartialContent::ReasoningSummary(text) => {
				ActionPayload::ReasoningSummary(ReasoningSummaryItem(text))
			}
			PartialContent::Delta(delta) => {
				// during streaming, a delta may arrive before the action
				// is created (eg OutputTextDelta before OutputItemAdded
				// has content). Use the key to determine the payload type.
				match key {
					PartialItemKey::ReasoningSummary { .. } => {
						ActionPayload::ReasoningSummary(
							ReasoningSummaryItem(delta),
						)
					}
					PartialItemKey::Single { .. } => {
						// single-key items are function calls
						ActionPayload::FunctionCall(FunctionCallItem {
							name: String::new(),
							arguments: delta,
						})
					}
					PartialItemKey::Content { .. } => {
						// default to text for content-keyed deltas
						ActionPayload::Text(TextItem(delta))
					}
				}
			}
			PartialContent::TextDone { text, .. } => {
				ActionPayload::Text(TextItem(text))
			}
			PartialContent::RefusalDone { refusal } => {
				ActionPayload::Refusal(RefusalItem(refusal))
			}
			PartialContent::ReasoningDone { content } => {
				ActionPayload::ReasoningContent(ReasoningContentItem(content))
			}
			PartialContent::AnnotationAdded { .. } => {
				bevybail!(
					"Cannot create payload from an annotation event alone"
				)
			}
			PartialContent::FunctionCallArgumentsDone(_) => {
				bevybail!("Cannot create payload from FunctionCallArgumentsDone without name and call_id context")
			}
		}
		.xok()
	}

	/// Apply a [`PartialContent`] to an existing [`ActionPayload`], mutating
	/// it in place. Handles deltas, replacements, and annotation inlining.
	fn apply_partial_content(
		&mut self,
		key: &PartialItemKey,
		partial: PartialContent,
		payload: &mut ActionPayload,
	) -> Result {
		match (partial, payload) {
			// OutputContent replaces matching payload wholesale
			(
				PartialContent::OutputContent(OutputContent::OutputText(t)),
				ActionPayload::Text(item),
			) => {
				item.0 = if t.annotations.is_empty() {
					t.text
				} else {
					self.inline_output_text_annotations(&t.text, &t.annotations)
				};
			}
			(
				PartialContent::OutputContent(OutputContent::Refusal(r)),
				ActionPayload::Refusal(item),
			) => {
				item.0 = r.refusal;
			}
			// ContentPart replaces matching payload wholesale
			(
				PartialContent::ContentPart(ContentPart::InputText(t)),
				ActionPayload::Text(item),
			) => {
				item.0 = t.text;
			}
			(
				PartialContent::ContentPart(ContentPart::OutputText(t)),
				ActionPayload::Text(item),
			) => {
				item.0 = if t.annotations.is_empty() {
					t.text
				} else {
					self.inline_output_text_annotations(&t.text, &t.annotations)
				};
			}
			(
				PartialContent::ContentPart(ContentPart::Refusal(r)),
				ActionPayload::Refusal(item),
			) => {
				item.0 = r.refusal;
			}
			(
				PartialContent::ContentPart(ContentPart::ReasoningText(r)),
				ActionPayload::ReasoningContent(item),
			) => {
				item.0 = r.text;
			}
			(
				PartialContent::ContentPart(ContentPart::SummaryText(s)),
				ActionPayload::ReasoningSummary(item),
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
				ActionPayload::FunctionCall(item),
			) => {
				item.name = name;
				item.arguments = arguments;
			}
			// FunctionCallOutput updates the output string and resolves call_id
			(
				PartialContent::FunctionCallOutput { output, call_id },
				ActionPayload::FunctionCallOutput(item),
			) => {
				item.output = output;
				// update the function_call_item reference if possible
				if let Ok(action_id) = self.get_action_id_for_call(&call_id) {
					item.function_call_item = action_id;
				}
			}
			// ReasoningContent/Summary replace in place
			(
				PartialContent::ReasoningContent(text),
				ActionPayload::ReasoningContent(item),
			) => {
				item.0 = text;
			}
			(
				PartialContent::ReasoningSummary(text),
				ActionPayload::ReasoningSummary(item),
			) => {
				item.0 = text;
			}
			// Deltas append to the appropriate payload type
			(PartialContent::Delta(delta), ActionPayload::Text(item)) => {
				item.0.push_str(&delta);
			}
			(PartialContent::Delta(delta), ActionPayload::Refusal(item)) => {
				item.0.push_str(&delta);
			}
			(
				PartialContent::Delta(delta),
				ActionPayload::ReasoningContent(item),
			) => {
				item.0.push_str(&delta);
			}
			(
				PartialContent::Delta(delta),
				ActionPayload::ReasoningSummary(item),
			) => {
				item.0.push_str(&delta);
			}
			(
				PartialContent::Delta(delta),
				ActionPayload::FunctionCall(item),
			) => {
				item.arguments.push_str(&delta);
			}
			// TextDone overwrites text and caches original for annotation re-rendering
			(
				PartialContent::TextDone { text, .. },
				ActionPayload::Text(item),
			) => {
				self.annotation_inliner
					.set_original_text(key.clone(), text.clone());
				item.0 = text;
			}
			// RefusalDone overwrites refusal payload
			(
				PartialContent::RefusalDone { refusal },
				ActionPayload::Refusal(item),
			) => {
				item.0 = refusal;
			}
			// ReasoningDone overwrites reasoning payload
			(
				PartialContent::ReasoningDone { content },
				ActionPayload::ReasoningContent(item),
			) => {
				item.0 = content;
			}
			// AnnotationAdded: re-render from original text with all accumulated annotations
			(
				PartialContent::AnnotationAdded { annotation, .. },
				ActionPayload::Text(item),
			) => {
				if let Some(rendered) =
					self.annotation_inliner.push_annotation(key, annotation)
				{
					item.0 = rendered;
				}
			}
			// FunctionCallArgumentsDone overwrites arguments
			(
				PartialContent::FunctionCallArgumentsDone(args),
				ActionPayload::FunctionCall(item),
			) => {
				item.arguments = args;
			}
			// Mismatched or unsupported combinations
			(partial, payload) => {
				bevybail!(
					"Cannot apply {:?} to {:?}",
					std::mem::discriminant(&partial),
					payload.kind()
				)
			}
		}
		Ok(())
	}
}
