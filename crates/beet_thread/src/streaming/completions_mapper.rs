//! Maps between [`async_openai::types::chat`] types and beet
//! [`ResponsePartial`] / [`PostPartial`] types.
//!
//! Analogous to [`o11s_mapper`] but targeting the
//! OpenAI Chat Completions API.
use crate::o11s::ReasoningEffort;
use crate::prelude::*;
use async_openai::types::chat::ChatCompletionMessageToolCall;
use async_openai::types::chat::ChatCompletionMessageToolCalls;
use async_openai::types::chat::ChatCompletionRequestAssistantMessage;
use async_openai::types::chat::ChatCompletionRequestAssistantMessageContent;
use async_openai::types::chat::ChatCompletionRequestDeveloperMessage;
use async_openai::types::chat::ChatCompletionRequestDeveloperMessageContent;
use async_openai::types::chat::ChatCompletionRequestMessage;
use async_openai::types::chat::ChatCompletionRequestMessageContentPartImage;
use async_openai::types::chat::ChatCompletionRequestSystemMessage;
use async_openai::types::chat::ChatCompletionRequestSystemMessageContent;
use async_openai::types::chat::ChatCompletionRequestToolMessage;
use async_openai::types::chat::ChatCompletionRequestToolMessageContent;
use async_openai::types::chat::ChatCompletionRequestUserMessage;
use async_openai::types::chat::ChatCompletionRequestUserMessageContent;
use async_openai::types::chat::ChatCompletionRequestUserMessageContentPart;
use async_openai::types::chat::ChatCompletionTool;
use async_openai::types::chat::ChatCompletionToolChoiceOption;
use async_openai::types::chat::ChatCompletionTools;
use async_openai::types::chat::CreateChatCompletionResponse;
use async_openai::types::chat::CreateChatCompletionStreamResponse;
use async_openai::types::chat::FinishReason;
use async_openai::types::chat::FunctionCall;
use async_openai::types::chat::FunctionObject;
use async_openai::types::chat::ImageUrl;
use async_openai::types::chat::ToolChoiceOptions;
use beet_core::prelude::*;

// ═══════════════════════════════════════════════════════════════════════
// Request Mapping: ModelPost -> ChatCompletionRequestMessage
// ═══════════════════════════════════════════════════════════════════════

/// Maps a [`ModelPost`] into a [`ChatCompletionRequestMessage`] by the one
/// rule every protocol shares (the model page, [`crate::table`], "Posts"):
/// a note as it is in the user role, another actor's call with its output as
/// one user message, the reading agent's own posts native, a System or
/// Developer post its role's message unwrapped, and anyone else's a user
/// message wrapped in `<post author=..>`. A tool message is text alone, so a
/// file that is neither an image nor text is one line naming it
/// ([`MediaView::line`]). A `media` post's bytes come from `blobs`, read
/// before.
pub fn post_to_completions_message(
	agent_id: &ActorId,
	post: ModelPost,
	blobs: &RequestBlobs,
) -> Result<ChatCompletionRequestMessage> {
	let user = |text: String, name: Option<String>| {
		ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
			content: ChatCompletionRequestUserMessageContent::Text(text),
			name,
		})
	};
	let post = match post {
		ModelPost::Post(post) => post,
		ModelPost::Note(note) => return user(note, None).xok(),
		ModelPost::Call { call, output } => {
			return user(
				call.call_text(output.as_ref()),
				Some(call.actor.name().to_string()),
			)
			.xok();
		}
	};
	let is_self = post.actor_id() == agent_id;
	let kind = post.actor.kind();
	let agent_post = post.post.as_agent_post().ok_or_else(|| {
		bevyhow!("log post {} is no model input", post.post.id())
	})?;

	let msg = match &agent_post {
		AgentPost::FunctionCall(fc) => {
			#[allow(deprecated)]
			let assistant = ChatCompletionRequestAssistantMessage {
				content: None,
				refusal: None,
				name: Some(post.actor.name().to_string()),
				audio: None,
				tool_calls: Some(vec![
					ChatCompletionMessageToolCalls::Function(
						ChatCompletionMessageToolCall {
							id: fc.call_id().to_string(),
							function: FunctionCall {
								name: fc.name().to_string(),
								arguments: fc.arguments().to_string(),
							},
						},
					),
				]),
				function_call: None,
			};
			ChatCompletionRequestMessage::Assistant(assistant)
		}
		AgentPost::FunctionCallOutput(fco) => {
			ChatCompletionRequestMessage::Tool(
				ChatCompletionRequestToolMessage {
					content: ChatCompletionRequestToolMessageContent::Text(
						fco.output().to_string(),
					),
					tool_call_id: fco.call_id().to_string(),
				},
			)
		}
		AgentPost::Url(view) if view.media_type().is_image() => {
			build_image_user_message(&post, view.url().to_string())
		}
		AgentPost::Url(view) if view.media_type().is_video() => {
			build_text_message(
				&post,
				&format!("[Video: {}]", view.url()),
				kind,
				is_self,
			)
		}
		AgentPost::Url(view) => {
			build_text_message(&post, view.url(), kind, is_self)
		}
		AgentPost::Media(view) => match blobs.bytes(post.post) {
			None => build_text_message(
				&post,
				RequestBlobs::UNAVAILABLE,
				kind,
				is_self,
			),
			Some(_) if view.media_type().is_image() => {
				build_image_user_message(
					&post,
					format!(
						"data:{};base64,{}",
						view.media_type().as_str(),
						blobs.base64(post.post).unwrap_or_default()
					),
				)
			}
			Some(bytes) if view.media_type().is_text() => {
				match std::str::from_utf8(&bytes) {
					Ok(text) => build_text_message(&post, text, kind, is_self),
					Err(_) => {
						build_text_message(&post, &view.line(), kind, is_self)
					}
				}
			}
			Some(_) => build_text_message(&post, &view.line(), kind, is_self),
		},
		AgentPost::Text(view)
		| AgentPost::Refusal(view)
		| AgentPost::Error(view)
		| AgentPost::ReasoningContent(view)
		| AgentPost::ReasoningSummary(view) => {
			build_text_message(&post, view.text(), kind, is_self)
		}
	};
	msg.xok()
}

/// Builds a text message, dispatching on role.
fn build_text_message(
	post: &AuthoredPost,
	text: &str,
	kind: ActorKind,
	is_self: bool,
) -> ChatCompletionRequestMessage {
	match kind {
		ActorKind::System => ChatCompletionRequestMessage::System(
			ChatCompletionRequestSystemMessage {
				content: ChatCompletionRequestSystemMessageContent::Text(
					text.to_string(),
				),
				name: Some(post.actor.name().to_string()),
			},
		),
		ActorKind::Developer => ChatCompletionRequestMessage::Developer(
			ChatCompletionRequestDeveloperMessage {
				content: ChatCompletionRequestDeveloperMessageContent::Text(
					text.to_string(),
				),
				name: Some(post.actor.name().to_string()),
			},
		),
		ActorKind::Agent if is_self => {
			// Own agent text -> assistant message
			#[allow(deprecated)]
			ChatCompletionRequestMessage::Assistant(
				ChatCompletionRequestAssistantMessage {
					content: Some(
						ChatCompletionRequestAssistantMessageContent::Text(
							text.to_string(),
						),
					),
					refusal: None,
					name: Some(post.actor.name().to_string()),
					audio: None,
					tool_calls: None,
					function_call: None,
				},
			)
		}
		// Agent (other) or Human -> user message with XML wrapping
		_ => {
			let wrapped = post.wrap_user_text(text);
			ChatCompletionRequestMessage::User(
				ChatCompletionRequestUserMessage {
					content: ChatCompletionRequestUserMessageContent::Text(
						wrapped,
					),
					name: Some(post.actor.name().to_string()),
				},
			)
		}
	}
}

/// Builds a user message containing an image content part.
fn build_image_user_message(
	post: &AuthoredPost,
	image_url: String,
) -> ChatCompletionRequestMessage {
	ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
		content: ChatCompletionRequestUserMessageContent::Array(vec![
			ChatCompletionRequestUserMessageContentPart::ImageUrl(
				ChatCompletionRequestMessageContentPartImage {
					image_url: ImageUrl {
						url: image_url,
						detail: None,
					},
				},
			),
		]),
		name: Some(post.actor.name().to_string()),
	})
}

// ═══════════════════════════════════════════════════════════════════════
// Response Mapping: CreateChatCompletionResponse -> ResponsePartial
// ═══════════════════════════════════════════════════════════════════════

/// Maps a non-streaming [`CreateChatCompletionResponse`] into a
/// [`ResponsePartial`].
pub fn response_to_partial(
	response: CreateChatCompletionResponse,
) -> Result<ResponsePartial> {
	let choice = response.choices.into_iter().next();

	let finish_reason = choice.as_ref().and_then(|choice| choice.finish_reason);

	let status = finish_reason_to_response_status(finish_reason);
	let post_status = finish_reason_to_post_status(finish_reason);

	let token_usage = response.usage.map(|usage| TokenUsage {
		input_tokens: usage.prompt_tokens,
		output_tokens: usage.completion_tokens,
		total_tokens: usage.total_tokens,
		cached_input_tokens: usage
			.prompt_tokens_details
			.and_then(|detail| detail.cached_tokens),
		reasoning_tokens: usage
			.completion_tokens_details
			.and_then(|detail| detail.reasoning_tokens),
	});

	let mut posts = Vec::new();

	if let Some(choice) = choice {
		let msg = choice.message;
		// Text content
		if let Some(content) = msg.content {
			if !content.is_empty() {
				posts.push(PostPartial {
					key: PostPartialKey::Content {
						responses_id: response.id.clone(),
						content_index: 0,
					},
					status: post_status,
					content: PartialContent::TextDone {
						text: content,
						logprobs: Vec::new(),
					},
				});
			}
		}
		// Refusal
		if let Some(refusal) = msg.refusal {
			if !refusal.is_empty() {
				posts.push(PostPartial {
					key: PostPartialKey::Content {
						responses_id: response.id.clone(),
						content_index: posts.len() as u32,
					},
					status: post_status,
					content: PartialContent::RefusalDone { refusal },
				});
			}
		}
		// Tool calls
		if let Some(tool_calls) = msg.tool_calls {
			for tc in tool_calls {
				if let ChatCompletionMessageToolCalls::Function(fc) = tc {
					posts.push(PostPartial {
						key: PostPartialKey::Single {
							responses_id: fc.id.clone(),
						},
						status: post_status,
						content: PartialContent::FunctionCall {
							name: fc.function.name,
							call_id: fc.id,
							arguments: fc.function.arguments,
						},
					});
				}
			}
		}
	}

	ResponsePartial {
		response_id: response.id,
		response_stored: false,
		status,
		token_usage,
		posts,
	}
	.xok()
}

// ═══════════════════════════════════════════════════════════════════════
// Streaming: StreamAccumulator + chunk mapping
// ═══════════════════════════════════════════════════════════════════════

/// Maps a single streaming chunk into a [`ResponsePartial`].
/// The `accumulated` state tracks tool calls that span multiple chunks.
pub fn stream_chunk_to_partial(
	chunk: CreateChatCompletionStreamResponse,
	accumulated: &mut StreamAccumulator,
) -> Result<ResponsePartial> {
	let choice = chunk.choices.into_iter().next();

	let finish_reason = choice.as_ref().and_then(|choice| choice.finish_reason);

	let status = finish_reason_to_response_status(finish_reason);
	let is_done = finish_reason.is_some();

	let token_usage = chunk.usage.map(|usage| TokenUsage {
		input_tokens: usage.prompt_tokens,
		output_tokens: usage.completion_tokens,
		total_tokens: usage.total_tokens,
		cached_input_tokens: usage
			.prompt_tokens_details
			.and_then(|detail| detail.cached_tokens),
		reasoning_tokens: usage
			.completion_tokens_details
			.and_then(|detail| detail.reasoning_tokens),
	});

	let mut posts = Vec::new();

	if let Some(choice) = choice {
		let delta = choice.delta;

		// Text content delta
		if let Some(content) = delta.content {
			if !content.is_empty() {
				posts.push(PostPartial {
					key: PostPartialKey::Content {
						responses_id: chunk.id.clone(),
						content_index: 0,
					},
					status: PostStatus::InProgress,
					content: PartialContent::Delta(content),
				});
			}
		}

		// Refusal delta
		if let Some(refusal) = delta.refusal {
			if !refusal.is_empty() {
				posts.push(PostPartial {
					key: PostPartialKey::Content {
						responses_id: chunk.id.clone(),
						content_index: 1,
					},
					status: PostStatus::InProgress,
					content: PartialContent::Delta(refusal),
				});
			}
		}

		// Tool call deltas — accumulate and emit
		if let Some(tool_call_chunks) = delta.tool_calls {
			for tc_chunk in tool_call_chunks {
				let acc = accumulated.get_or_insert(tc_chunk.index);

				if let Some(id) = tc_chunk.id {
					acc.id = id;
				}
				if let Some(ref func) = tc_chunk.function {
					if let Some(ref name) = func.name {
						acc.name = name.clone();
					}
					if let Some(ref args) = func.arguments {
						acc.arguments.push_str(args);
					}
				}

				// Emit a delta for the arguments fragment
				let args_delta = tc_chunk
					.function
					.and_then(|func| func.arguments)
					.unwrap_or_default();
				if !args_delta.is_empty() {
					posts.push(PostPartial {
						key: PostPartialKey::Single {
							responses_id: acc.id.clone(),
						},
						status: PostStatus::InProgress,
						content: PartialContent::Delta(args_delta),
					});
				}
			}
		}
	}

	// On finish, emit completed tool call partials from accumulator
	if is_done {
		for acc_tc in accumulated.tool_calls.drain(..) {
			if !acc_tc.id.is_empty() {
				posts.push(PostPartial {
					key: PostPartialKey::Single {
						responses_id: acc_tc.id.clone(),
					},
					status: PostStatus::Completed,
					content: PartialContent::FunctionCall {
						name: acc_tc.name,
						call_id: acc_tc.id,
						arguments: acc_tc.arguments,
					},
				});
			}
		}
	}

	ResponsePartial {
		response_id: chunk.id,
		response_stored: false,
		status,
		token_usage,
		posts,
	}
	.xok()
}

// ═══════════════════════════════════════════════════════════════════════
// Tool Mapping
// ═══════════════════════════════════════════════════════════════════════

/// Maps a beet [`ToolDefinition`] to a [`ChatCompletionTools`].
///
/// # Errors
///
/// Errors if a function's parameter schema does not convert into the json
/// value async-openai holds, which it always does short of a byte string.
pub fn tool_to_completions_tool(
	tool: &ToolDefinition,
) -> Result<ChatCompletionTools> {
	match tool {
		ToolDefinition::Function(func) => {
			let mut params = func.params_schema().clone();
			params.sanitize_for_strict_mode();
			ChatCompletionTools::Function(ChatCompletionTool {
				function: FunctionObject {
					name: FunctionToolDefinition::encode_name(func.path()),
					description: Some(func.description().to_string()),
					// async-openai's own json value, inferred from the field
					parameters: Some(params.into_inner().into_serde()?),
					strict: Some(true),
				},
			})
		}
		ToolDefinition::Provider(provider) => {
			// Provider tools don't have a direct completions analogue;
			// map as a function with no parameters.
			ChatCompletionTools::Function(ChatCompletionTool {
				function: FunctionObject {
					name: provider.name().to_string(),
					description: None,
					parameters: None,
					strict: None,
				},
			})
		}
	}
	.xok()
}

/// Maps a beet [`ToolChoice`] to a [`ChatCompletionToolChoiceOption`].
pub fn tool_choice_to_completions(
	choice: &ToolChoice,
) -> ChatCompletionToolChoiceOption {
	use async_openai::types::chat::ChatCompletionNamedToolChoice;
	use async_openai::types::chat::FunctionName;

	match choice {
		ToolChoice::Auto => {
			ChatCompletionToolChoiceOption::Mode(ToolChoiceOptions::Auto)
		}
		ToolChoice::None => {
			ChatCompletionToolChoiceOption::Mode(ToolChoiceOptions::None)
		}
		ToolChoice::RequiredAny => {
			ChatCompletionToolChoiceOption::Mode(ToolChoiceOptions::Required)
		}
		ToolChoice::RequiredList(paths) => {
			// The completions API can only force a single named function;
			// pick the first and rely on the caller to be specific.
			if let Some(path) = paths.first() {
				ChatCompletionToolChoiceOption::Function(
					ChatCompletionNamedToolChoice {
						function: FunctionName {
							name: FunctionToolDefinition::encode_name(path),
						},
					},
				)
			} else {
				ChatCompletionToolChoiceOption::Mode(
					ToolChoiceOptions::Required,
				)
			}
		}
		ToolChoice::AutoList(paths) => {
			if let Some(path) = paths.first() {
				ChatCompletionToolChoiceOption::Function(
					ChatCompletionNamedToolChoice {
						function: FunctionName {
							name: FunctionToolDefinition::encode_name(path),
						},
					},
				)
			} else {
				ChatCompletionToolChoiceOption::Mode(ToolChoiceOptions::Auto)
			}
		}
	}
}

/// Map the shared [`ReasoningEffort`] onto the completions wire type.
///
/// The wire type carries provider-specific variants the OpenResponses spec does
/// not (`minimal`, an OpenAI-only effort for models predating `none`); the shared
/// enum stays spec-shaped and this mapper is where a provider extension would be
/// reintroduced, per-provider, if a configured model ever needed one.
pub fn reasoning_effort(
	effort: ReasoningEffort,
) -> async_openai::types::chat::ReasoningEffort {
	use async_openai::types::chat::ReasoningEffort as Wire;
	match effort {
		ReasoningEffort::None => Wire::None,
		ReasoningEffort::Low => Wire::Low,
		ReasoningEffort::Medium => Wire::Medium,
		ReasoningEffort::High => Wire::High,
		ReasoningEffort::Xhigh => Wire::Xhigh,
	}
}

// ═══════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════

fn finish_reason_to_response_status(
	reason: Option<FinishReason>,
) -> ResponseStatus {
	match reason {
		None => ResponseStatus::InProgress,
		Some(FinishReason::Stop) => ResponseStatus::Completed,
		Some(FinishReason::Length) => {
			ResponseStatus::Incomplete(Some("max_tokens".to_string()))
		}
		Some(FinishReason::ToolCalls) => ResponseStatus::Completed,
		Some(FinishReason::ContentFilter) => {
			ResponseStatus::Incomplete(Some("content_filter".to_string()))
		}
		Some(FinishReason::FunctionCall) => ResponseStatus::Completed,
	}
}

fn finish_reason_to_post_status(reason: Option<FinishReason>) -> PostStatus {
	match reason {
		None => PostStatus::InProgress,
		Some(FinishReason::Stop)
		| Some(FinishReason::ToolCalls)
		| Some(FinishReason::FunctionCall) => PostStatus::Completed,
		Some(FinishReason::Length) | Some(FinishReason::ContentFilter) => {
			PostStatus::Interrupted
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use async_openai::types::chat::ChatCompletionRequestMessage;
	use async_openai::types::chat::ChatCompletionRequestUserMessageContent;
	use beet_core::prelude::*;

	/// The text of the user message a post by a person maps to.
	fn user_text(post: Post) -> String {
		let mut records = ThreadRecords::new();
		records.insert_actor(Actor::user());
		records.upsert_post(post);
		let item = records
			.model_posts(&ActorId::AGENT, None)
			.into_iter()
			.next()
			.unwrap();
		match completions_mapper::post_to_completions_message(
			&ActorId::AGENT,
			item,
			&RequestBlobs::default(),
		)
		.unwrap()
		{
			ChatCompletionRequestMessage::User(message) => {
				match message.content {
					ChatCompletionRequestUserMessageContent::Text(text) => text,
					other => panic!("expected text, found {other:?}"),
				}
			}
			other => panic!("expected a user message, found {other:?}"),
		}
	}

	/// A tool message is text alone, so a file that is neither an image nor
	/// text is one line naming it, a text file its text.
	#[beet_core::test]
	fn a_file_completions_cannot_carry_is_a_line() {
		let file = |media_type: MediaType, bytes: &[u8], stem: &str| {
			AgentPost::new_media(
				ActorId::USER,
				ThreadId::now(),
				media_type,
				bytes.to_vec(),
				Some(stem.into()),
				PostStatus::Completed,
			)
		};
		user_text(file(MediaType::Pdf, &[0; 2048], "report")).xpect_contains(
			">[file: report.pdf, application/pdf, 2 KB]</post>",
		);
		user_text(file(MediaType::Markdown, b"# notes", "notes"))
			.xpect_contains("># notes</post>");
	}

	/// A url names its media type, so an image url reaches each protocol as
	/// an image, and a video url completions as its line.
	#[beet_core::test]
	fn an_image_url_is_an_image() {
		let url = |url: &str| {
			AgentPost::new_url(
				ActorId::USER,
				ThreadId::now(),
				url,
				MediaType::from_path(url),
				None,
				PostStatus::Completed,
			)
		};
		let mut records = ThreadRecords::new();
		records.insert_actor(Actor::user());
		records.upsert_post(url("https://example.com/cat.png"));
		let item = || {
			records
				.model_posts(&ActorId::AGENT, None)
				.into_iter()
				.next()
				.unwrap()
		};
		let blobs = RequestBlobs::default();
		Value::from_serde(
			&completions_mapper::post_to_completions_message(
				&ActorId::AGENT,
				item(),
				&blobs,
			)
			.unwrap(),
		)
		.unwrap()["content"][0]["image_url"]["url"]
			.as_str()
			.unwrap()
			.xpect_eq("https://example.com/cat.png");
		Value::from_serde(
			&o11s_mapper::post_to_o11s_input(&ActorId::AGENT, item(), &blobs)
				.unwrap(),
		)
		.unwrap()["content"][0]["type"]
			.as_str()
			.unwrap()
			.xpect_eq("input_image");
		user_text(url("https://example.com/clip.mp4"))
			.xpect_contains(">[Video: https://example.com/clip.mp4]</post>");
	}
}
