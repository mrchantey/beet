use crate::o11s::Annotation;
use crate::o11s::ContentPart;
use crate::o11s::OutputContent;
use crate::prelude::*;
use beet_core::prelude::*;

#[derive(Debug, Default)]
pub struct ActionPartialMap {
	annotation_inliner: AnnotationInliner,
	/// Map partial item keys to the created [`ActionId`]
	action_key_map: HashMap<ActionPartialKey, ActionId>,
}

impl ActionPartialMap {
	pub fn new() -> Self { Self::default() }

	pub fn with_annotation_inliner(
		mut self,
		inliner: AnnotationInliner,
	) -> Self {
		self.annotation_inliner = inliner;
		self
	}


	/// Apply a stream of partial items against the action map, producing
	/// [`ActionChanges`] describing what was created or modified.
	/// Also registers call_id and response_item mappings for new actions.
	pub fn apply_actions(
		&mut self,
		actions: &mut DocMap<Action>,
		author: ActorId,
		thread: ThreadId,
		partial_items: impl IntoIterator<Item = ActionPartial>,
	) -> Result<ActionChanges> {
		let mut changes = ActionChanges::default();
		for partial_item in partial_items {
			let ActionPartial {
				key,
				status,
				content,
			} = partial_item;
			if let Some(&action_id) = self.action_key_map.get(&key) {
				// action already exists, update it in place
				let action = actions.get_mut(action_id)?;
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
				}
			} else {
				// create new action
				let payload =
					self.partial_content_into_payload(actions, &key, content)?;
				let action = Action::new(author, thread, status, payload);
				let action_id = action.id();

				// register response key -> action id
				self.set_response_item(key, action_id)?;

				changes.created.push(action_id);
				actions.insert(action);
			}
		}
		changes.xok()
	}


	// ═══════════════════════════════════════════════════════════════════════
	// Response item key mapping
	// ═══════════════════════════════════════════════════════════════════════

	fn set_response_item(
		&mut self,
		key: ActionPartialKey,
		action_id: ActionId,
	) -> Result {
		if self.action_key_map.contains_key(&key) {
			bevybail!("action_key_map already has an entry for key {key:?}");
		} else {
			self.action_key_map.insert(key, action_id);
		}
		Ok(())
	}

	fn get_response_item(&self, key: &ActionPartialKey) -> Result<ActionId> {
		self.action_key_map.get(key).cloned().ok_or_else(|| {
			bevyhow!("no action_id registered for responses item key {key:?}")
		})
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
		actions: &mut DocMap<Action>,
		key: &ActionPartialKey,
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
				call_id,
			} => ActionPayload::FunctionCall(FunctionCallItem {
				name,
				arguments,
				call_id
			}),
			PartialContent::FunctionCallOutput { call_id, output } => {
				ActionPayload::FunctionCallOutput(FunctionCallOutputItem {
					output,
					call_id
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
					ActionPartialKey::ReasoningSummary { .. } => {
						ActionPayload::ReasoningSummary(
							ReasoningSummaryItem(delta),
						)
					}
					ActionPartialKey::Single { .. } => {
						let action_id = self.get_response_item(key)?;
						let action = actions.get(action_id)?;
						let ActionPayload::FunctionCall(item) = action.payload() else {
							bevybail!(
								"Expected FunctionCall payload for key {:?}, found {:?}",
								key,
								action.payload().kind()
							)
						};
						// single-key items are function calls
						ActionPayload::FunctionCall(FunctionCallItem {
							name: String::new(),
							arguments: delta,
							call_id: item.call_id.clone()
						})
					}
					ActionPartialKey::Content { .. } => {
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
		key: &ActionPartialKey,
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
				item.call_id = call_id;
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
