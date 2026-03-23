use crate::o11s::Annotation;
use crate::o11s::ContentPart;
use crate::o11s::OutputContent;
use crate::prelude::*;
use beet_core::prelude::*;

#[derive(Debug, Default)]
pub struct PostPartialMap {
	annotation_inliner: AnnotationInliner,
	/// Map partial item keys to the created [`PostId`]
	post_key_map: HashMap<PostPartialKey, PostId>,
}

impl PostPartialMap {
	pub fn new() -> Self { Self::default() }

	pub fn with_annotation_inliner(
		mut self,
		inliner: AnnotationInliner,
	) -> Self {
		self.annotation_inliner = inliner;
		self
	}


	/// Apply a stream of partial items against the post map, producing
	/// [`PostChanges`] describing what was created or modified.
	/// Also registers call_id and response_item mappings for new posts.
	pub fn apply_posts(
		&mut self,
		posts: &mut DocMap<Post>,
		author: UserId,
		thread: ThreadId,
		partial_items: impl IntoIterator<Item = PostPartial>,
	) -> Result<PostChanges> {
		let mut changes = PostChanges::default();
		for partial_item in partial_items {
			let PostPartial {
				key,
				status,
				content,
			} = partial_item;
			if let Some(&post_id) = self.post_key_map.get(&key) {
				// post already exists, update it in place
				let post = posts.get_mut(post_id)?;
				let before_mutation = post.hash();

				post.set_status(status);
				self.apply_partial_content(&key, content, post.payload_mut())?;
				let after_mutation = post.hash();
				if before_mutation != after_mutation {
					changes.modified.push(post.clone());
				}
			} else {
				// create new post
				let payload =
					self.partial_content_into_payload(posts, &key, content)?;
				let post = Post::new(author, thread, status, payload);
				let post_id = post.id();

				// register response key -> post id
				self.set_post_key(key, post_id)?;

				changes.created.push(post.clone());
				posts.insert(post);
			}
		}
		changes.xok()
	}


	// ═══════════════════════════════════════════════════════════════════════
	// Response item key mapping
	// ═══════════════════════════════════════════════════════════════════════

	fn set_post_key(&mut self, key: PostPartialKey, post_id: PostId) -> Result {
		if self.post_key_map.contains_key(&key) {
			bevybail!("post_key_map already has an entry for key {key:?}");
		} else {
			self.post_key_map.insert(key, post_id);
		}
		Ok(())
	}

	fn get_response_item(&self, key: &PostPartialKey) -> Result<PostId> {
		self.post_key_map.get(key).cloned().ok_or_else(|| {
			bevyhow!("no post_id registered for responses item key {key:?}")
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

	/// Convert a [`PartialContent`] into a finalized [`PostPayload`], inlining
	/// annotations as markdown where appropriate.
	/// The key is used to disambiguate content types for generic variants
	/// like [`PartialContent::Delta`].
	fn partial_content_into_payload(
		&self,
		posts: &mut DocMap<Post>,
		key: &PostPartialKey,
		partial: PartialContent,
	) -> Result<PostPayload> {
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
				PostPayload::Text(TextItem(text))
			}
			PartialContent::OutputContent(OutputContent::Refusal(r)) => {
				PostPayload::Refusal(RefusalItem(r.refusal))
			}
			PartialContent::ContentPart(ContentPart::InputText(t)) => {
				PostPayload::Text(TextItem(t.text))
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
				PostPayload::Text(TextItem(text))
			}
			PartialContent::ContentPart(ContentPart::Refusal(r)) => {
				PostPayload::Refusal(RefusalItem(r.refusal))
			}
			PartialContent::ContentPart(ContentPart::ReasoningText(r)) => {
				PostPayload::ReasoningContent(ReasoningContentItem(r.text))
			}
			PartialContent::ContentPart(ContentPart::SummaryText(s)) => {
				PostPayload::ReasoningSummary(ReasoningSummaryItem(s.text))
			}
			PartialContent::ContentPart(ContentPart::InputImage(img)) => {
				PostPayload::Url(UrlItem {
					file_stem: None,
					media_type: MediaType::from_path(&img.image_url),
					url: img.image_url.into(),
				})
			}
			PartialContent::ContentPart(ContentPart::InputVideo(vid)) => {
				PostPayload::Url(UrlItem {
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
					PostPayload::Url(UrlItem {
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
					PostPayload::Bytes(BytesItem {
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
			} => PostPayload::FunctionCall(FunctionCallItem {
				name,
				arguments,
				call_id
			}),
			PartialContent::FunctionCallOutput { call_id, output } => {
				PostPayload::FunctionCallOutput(FunctionCallOutputItem {
					output,
					call_id
				})
			}
			PartialContent::ReasoningContent(text) => {
				PostPayload::ReasoningContent(ReasoningContentItem(text))
			}
			PartialContent::ReasoningSummary(text) => {
				PostPayload::ReasoningSummary(ReasoningSummaryItem(text))
			}
			PartialContent::Delta(delta) => {
				// during streaming, a delta may arrive before the post
				// is created (eg OutputTextDelta before OutputItemAdded
				// has content). Use the key to determine the payload type.
				match key {
					PostPartialKey::ReasoningSummary { .. } => {
						PostPayload::ReasoningSummary(
							ReasoningSummaryItem(delta),
						)
					}
					PostPartialKey::Single { .. } => {
						let post_id = self.get_response_item(key)?;
						let post = posts.get(post_id)?;
						let PostPayload::FunctionCall(item) = post.payload() else {
							bevybail!(
								"Expected FunctionCall payload for key {:?}, found {:?}",
								key,
								post.payload().kind()
							)
						};
						// single-key items are function calls
						PostPayload::FunctionCall(FunctionCallItem {
							name: String::new(),
							arguments: delta,
							call_id: item.call_id.clone()
						})
					}
					PostPartialKey::Content { .. } => {
						// default to text for content-keyed deltas
						PostPayload::Text(TextItem(delta))
					}
				}
			}
			PartialContent::TextDone { text, .. } => {
				PostPayload::Text(TextItem(text))
			}
			PartialContent::RefusalDone { refusal } => {
				PostPayload::Refusal(RefusalItem(refusal))
			}
			PartialContent::ReasoningDone { content } => {
				PostPayload::ReasoningContent(ReasoningContentItem(content))
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

	/// Apply a [`PartialContent`] to an existing [`PostPayload`], mutating
	/// it in place. Handles deltas, replacements, and annotation inlining.
	fn apply_partial_content(
		&mut self,
		key: &PostPartialKey,
		partial: PartialContent,
		payload: &mut PostPayload,
	) -> Result {
		match (partial, payload) {
			// OutputContent replaces matching payload wholesale
			(
				PartialContent::OutputContent(OutputContent::OutputText(t)),
				PostPayload::Text(item),
			) => {
				item.0 = if t.annotations.is_empty() {
					t.text
				} else {
					self.inline_output_text_annotations(&t.text, &t.annotations)
				};
			}
			(
				PartialContent::OutputContent(OutputContent::Refusal(r)),
				PostPayload::Refusal(item),
			) => {
				item.0 = r.refusal;
			}
			// ContentPart replaces matching payload wholesale
			(
				PartialContent::ContentPart(ContentPart::InputText(t)),
				PostPayload::Text(item),
			) => {
				item.0 = t.text;
			}
			(
				PartialContent::ContentPart(ContentPart::OutputText(t)),
				PostPayload::Text(item),
			) => {
				item.0 = if t.annotations.is_empty() {
					t.text
				} else {
					self.inline_output_text_annotations(&t.text, &t.annotations)
				};
			}
			(
				PartialContent::ContentPart(ContentPart::Refusal(r)),
				PostPayload::Refusal(item),
			) => {
				item.0 = r.refusal;
			}
			(
				PartialContent::ContentPart(ContentPart::ReasoningText(r)),
				PostPayload::ReasoningContent(item),
			) => {
				item.0 = r.text;
			}
			(
				PartialContent::ContentPart(ContentPart::SummaryText(s)),
				PostPayload::ReasoningSummary(item),
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
				PostPayload::FunctionCall(item),
			) => {
				item.name = name;
				item.arguments = arguments;
			}
			// FunctionCallOutput updates the output string and resolves call_id
			(
				PartialContent::FunctionCallOutput { output, call_id },
				PostPayload::FunctionCallOutput(item),
			) => {
				item.output = output;
				item.call_id = call_id;
			}
			// ReasoningContent/Summary replace in place
			(
				PartialContent::ReasoningContent(text),
				PostPayload::ReasoningContent(item),
			) => {
				item.0 = text;
			}
			(
				PartialContent::ReasoningSummary(text),
				PostPayload::ReasoningSummary(item),
			) => {
				item.0 = text;
			}
			// Deltas append to the appropriate payload type
			(PartialContent::Delta(delta), PostPayload::Text(item)) => {
				item.0.push_str(&delta);
			}
			(PartialContent::Delta(delta), PostPayload::Refusal(item)) => {
				item.0.push_str(&delta);
			}
			(
				PartialContent::Delta(delta),
				PostPayload::ReasoningContent(item),
			) => {
				item.0.push_str(&delta);
			}
			(
				PartialContent::Delta(delta),
				PostPayload::ReasoningSummary(item),
			) => {
				item.0.push_str(&delta);
			}
			(PartialContent::Delta(delta), PostPayload::FunctionCall(item)) => {
				item.arguments.push_str(&delta);
			}
			// TextDone overwrites text and caches original for annotation re-rendering
			(
				PartialContent::TextDone { text, .. },
				PostPayload::Text(item),
			) => {
				self.annotation_inliner
					.set_original_text(key.clone(), text.clone());
				item.0 = text;
			}
			// RefusalDone overwrites refusal payload
			(
				PartialContent::RefusalDone { refusal },
				PostPayload::Refusal(item),
			) => {
				item.0 = refusal;
			}
			// ReasoningDone overwrites reasoning payload
			(
				PartialContent::ReasoningDone { content },
				PostPayload::ReasoningContent(item),
			) => {
				item.0 = content;
			}
			// AnnotationAdded: re-render from original text with all accumulated annotations
			(
				PartialContent::AnnotationAdded { annotation, .. },
				PostPayload::Text(item),
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
				PostPayload::FunctionCall(item),
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
