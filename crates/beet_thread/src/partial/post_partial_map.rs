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
		author: ActorId,
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
				let post = posts.get_mut(post_id)?;
				let before_mutation = post.hash_self();

				post.set_status(status);
				self.apply_partial_content(&key, content, post)?;
				let after_mutation = post.hash_self();
				if before_mutation != after_mutation {
					changes.modified.push(post.clone());
				}
			} else {
				// create new post
				let post = self.partial_content_into_post(
					posts, &key, content, author, thread, status,
				)?;
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
	// Annotation helpers
	// ═══════════════════════════════════════════════════════════════════════

	fn inline_output_text_annotations(
		&self,
		text: &str,
		annotations: &[Annotation],
	) -> String {
		self.annotation_inliner
			.inline_annotations(text, annotations)
	}

	// ═══════════════════════════════════════════════════════════════════════
	// Post creation from partial content
	// ═══════════════════════════════════════════════════════════════════════

	/// Convert a [`PartialContent`] into a new [`Post`], inlining
	/// annotations as markdown where appropriate.
	/// The key is used to disambiguate content types for generic variants
	/// like [`PartialContent::Delta`].
	fn partial_content_into_post(
		&self,
		posts: &mut DocMap<Post>,
		key: &PostPartialKey,
		partial: PartialContent,
		author: ActorId,
		thread: ThreadId,
		status: PostStatus,
	) -> Result<Post> {
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
				AgentPost::new_text(author, thread, text, status)
			}
			PartialContent::OutputContent(OutputContent::Refusal(r)) => {
				AgentPost::new_refusal(author, thread, r.refusal, status)
			}
			PartialContent::ContentPart(ContentPart::InputText(t)) => {
				AgentPost::new_text(author, thread, t.text, status)
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
				AgentPost::new_text(author, thread, text, status)
			}
			PartialContent::ContentPart(ContentPart::Refusal(r)) => {
				AgentPost::new_refusal(author, thread, r.refusal, status)
			}
			PartialContent::ContentPart(ContentPart::ReasoningText(r)) => {
				AgentPost::new_reasoning(author, thread, r.text, status)
			}
			PartialContent::ContentPart(ContentPart::SummaryText(s)) => {
				AgentPost::new_reasoning_summary(
					author, thread, s.text, status,
				)
			}
			PartialContent::ContentPart(ContentPart::InputImage(img)) => {
				AgentPost::new_url(
					author, thread, img.image_url, None, status,
				)
			}
			PartialContent::ContentPart(ContentPart::InputVideo(vid)) => {
				AgentPost::new_url(
					author, thread, vid.video_url, None, status,
				)
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
					AgentPost::new_url(
						author, thread, url, file_stem, status,
					)
				} else if let Some(data) = file.file_data {
					use base64::Engine;
					let bytes = base64::prelude::BASE64_STANDARD
						.decode(&data)
						.map_err(|err| {
							bevyhow!(
								"Failed to decode base64 file data: {err}"
							)
						})?;
					AgentPost::new_bytes(
						author, thread, media_type, bytes, file_stem, status,
					)
				} else {
					bevybail!("InputFile has neither file_url nor file_data")
				}
			}
			PartialContent::FunctionCall {
				name,
				call_id,
				arguments,
			} => AgentPost::new_function_call(
				author, thread, name, call_id, arguments, status,
			),
			PartialContent::FunctionCallOutput { call_id, output } => {
				AgentPost::new_function_call_output(
					author, thread, call_id, output, None, status,
				)
			}
			PartialContent::ReasoningContent(text) => {
				AgentPost::new_reasoning(author, thread, text, status)
			}
			PartialContent::ReasoningSummary(text) => {
				AgentPost::new_reasoning_summary(author, thread, text, status)
			}
			PartialContent::Delta(delta) => {
				// During streaming, a delta may arrive before the post
				// is created (eg OutputTextDelta before OutputItemAdded
				// has content). Use the key to determine the post type.
				match key {
					PostPartialKey::ReasoningSummary { .. } => {
						AgentPost::new_reasoning_summary(
							author, thread, delta, status,
						)
					}
					PostPartialKey::Single { .. } => {
						// Single-key items are function calls; look up the
						// existing post to get the call_id.
						let post_id = self.get_response_item(key)?;
						let post = posts.get(post_id)?;
						let agent_post = post.as_agent_post();
						let fc =
							agent_post.as_function_call().ok_or_else(|| {
								bevyhow!(
									"Expected FunctionCall post for key {:?}",
									key,
								)
							})?;
						AgentPost::new_function_call(
							author,
							thread,
							"",
							fc.call_id(),
							delta,
							status,
						)
					}
					PostPartialKey::Content { .. } => {
						// default to text for content-keyed deltas
						AgentPost::new_text(author, thread, delta, status)
					}
				}
			}
			PartialContent::TextDone { text, .. } => {
				AgentPost::new_text(author, thread, text, status)
			}
			PartialContent::RefusalDone { refusal } => {
				AgentPost::new_refusal(author, thread, refusal, status)
			}
			PartialContent::ReasoningDone { content } => {
				AgentPost::new_reasoning(author, thread, content, status)
			}
			PartialContent::AnnotationAdded { .. } => {
				bevybail!(
					"Cannot create post from an annotation event alone"
				)
			}
			PartialContent::FunctionCallArgumentsDone(_) => {
				bevybail!("Cannot create post from FunctionCallArgumentsDone without name and call_id context")
			}
		}
		.xok()
	}

	// ═══════════════════════════════════════════════════════════════════════
	// Mutation of existing posts from partial content
	// ═══════════════════════════════════════════════════════════════════════

	/// Apply a [`PartialContent`] to an existing [`Post`], mutating
	/// it in place. Handles deltas, replacements, and annotation inlining.
	fn apply_partial_content(
		&mut self,
		key: &PostPartialKey,
		partial: PartialContent,
		post: &mut Post,
	) -> Result {
		match partial {
			// ── OutputContent replaces matching post wholesale ───────
			PartialContent::OutputContent(OutputContent::OutputText(t)) => {
				let text = if t.annotations.is_empty() {
					t.text
				} else {
					self.inline_output_text_annotations(&t.text, &t.annotations)
				};
				post.set_text(text);
			}
			PartialContent::OutputContent(OutputContent::Refusal(r)) => {
				post.set_text(r.refusal);
			}

			// ── ContentPart replaces matching post wholesale ────────
			PartialContent::ContentPart(ContentPart::InputText(t)) => {
				post.set_text(t.text);
			}
			PartialContent::ContentPart(ContentPart::OutputText(t)) => {
				let text = if t.annotations.is_empty() {
					t.text
				} else {
					self.inline_output_text_annotations(&t.text, &t.annotations)
				};
				post.set_text(text);
			}
			PartialContent::ContentPart(ContentPart::Refusal(r)) => {
				post.set_text(r.refusal);
			}
			PartialContent::ContentPart(ContentPart::ReasoningText(r)) => {
				post.set_text(r.text);
			}
			PartialContent::ContentPart(ContentPart::SummaryText(s)) => {
				post.set_text(s.text);
			}

			// ── FunctionCall updates name and arguments ─────────────
			PartialContent::FunctionCall {
				name,
				arguments,
				call_id: _,
			} => {
				post.set_text(arguments);
				post.metadata_mut()["fc_name"] =
					serde_json::Value::String(name);
			}

			// ── FunctionCallOutput updates output and call_id ───────
			PartialContent::FunctionCallOutput { output, call_id } => {
				post.set_text(output);
				post.metadata_mut()["fc_id"] =
					serde_json::Value::String(call_id);
			}

			// ── ReasoningContent/Summary replace in place ───────────
			PartialContent::ReasoningContent(text) => {
				post.set_text(text);
			}
			PartialContent::ReasoningSummary(text) => {
				post.set_text(text);
			}

			// ── Deltas append to the appropriate post type ──────────
			PartialContent::Delta(delta) => {
				// function call deltas append to arguments
				// text, refusal, reasoning content/summary all
				// store their content as text body
				// either way, all are just appending to the body
				post.push_str(&delta);
			}

			// ── TextDone overwrites text and caches original ────────
			PartialContent::TextDone { text, .. } => {
				self.annotation_inliner
					.set_original_text(key.clone(), text.clone());
				post.set_text(text);
			}

			// ── RefusalDone overwrites refusal body ─────────────────
			PartialContent::RefusalDone { refusal } => {
				post.set_text(refusal);
			}

			// ── ReasoningDone overwrites reasoning body ─────────────
			PartialContent::ReasoningDone { content } => {
				post.set_text(content);
			}

			// ── AnnotationAdded: re-render from original text ───────
			PartialContent::AnnotationAdded { annotation, .. } => {
				if let Some(rendered) =
					self.annotation_inliner.push_annotation(key, annotation)
				{
					post.set_text(rendered);
				}
			}

			// ── FunctionCallArgumentsDone overwrites arguments ──────
			PartialContent::FunctionCallArgumentsDone(args) => {
				post.set_text(args);
			}

			// ── Remaining ContentPart variants (images, files, etc)
			// are not expected as mutations to existing posts.
			other => {
				bevybail!(
					"Cannot apply {:?} to existing post",
					std::mem::discriminant(&other),
				)
			}
		}
		Ok(())
	}
}
