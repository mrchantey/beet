use crate::o11s::ContentPart;
use crate::o11s::MessageRole;
use crate::o11s::ResponseBody;
use crate::o11s::StreamingEvent;
use crate::o11s::request::FunctionCallOutputParam;
use crate::o11s::request::FunctionCallParam;
use crate::o11s::request::FunctionOutputContent;
use crate::o11s::request::InputItem;
use crate::o11s::request::MessageContent;
use crate::o11s::request::MessageParam;
use crate::o11s::streaming::ResponseCompletedEvent;
use crate::o11s::streaming::ResponseCreatedEvent;
use crate::o11s::streaming::ResponseFailedEvent;
use crate::o11s::streaming::ResponseInProgressEvent;
use crate::o11s::streaming::ResponseIncompleteEvent;
use crate::o11s::streaming::ResponseQueuedEvent;
use crate::prelude::*;
use beet_core::prelude::*;


/// Maps a [`PostView`] into an OpenResponses [`InputItem`] for request building.
/// Classification is performed via [`AgentPost`] for clean dispatch.
pub fn post_to_o11s_input(
	agent_id: ActorId,
	post: PostView,
) -> Result<o11s::request::InputItem> {
	let role = match post.actor.kind() {
		ActorKind::System => MessageRole::System,
		ActorKind::Developer => MessageRole::Developer,
		ActorKind::Agent => {
			if post.actor_id() == agent_id {
				MessageRole::Assistant
			} else {
				MessageRole::User
			}
		}
		ActorKind::Human => MessageRole::User,
	};

	let agent_post = post.post.as_agent_post();

	let input_item = match &agent_post {
		AgentPost::FunctionCall(fc) => {
			InputItem::FunctionCall(FunctionCallParam {
				id: None,
				call_id: fc.call_id().to_string(),
				name: fc.name().to_string(),
				arguments: fc.arguments().to_string(),
				status: None,
			})
		}
		AgentPost::FunctionCallOutput(fco) => {
			InputItem::FunctionCallOutput(FunctionCallOutputParam {
				id: None,
				call_id: fco.call_id().to_string(),
				output: FunctionOutputContent::Text(fco.output().to_string()),
				status: None,
			})
		}
		AgentPost::Url(url_view) => InputItem::Message(MessageParam {
			id: None,
			role,
			content: MessageContent::Parts(vec![ContentPart::InputFile(
				o11s::InputFile {
					filename: url_view.filename(),
					file_data: None,
					file_url: Some(url_view.url().to_string()),
				},
			)]),
			status: None,
		}),
		AgentPost::Bytes(bytes_view) => InputItem::Message(MessageParam {
			id: None,
			role,
			content: MessageContent::Parts(vec![ContentPart::InputFile(
				o11s::InputFile {
					filename: bytes_view.filename(),
					file_data: Some(bytes_view.body_base64()),
					file_url: None,
				},
			)]),
			status: None,
		}),
		// Text-like posts: Text, Refusal, ReasoningContent, ReasoningSummary, Error
		_ => {
			let value = post.body_str()?;
			// using xml on assistant messages is likely to confuse them,
			// ie they start using them in their own responses
			let text = if role != MessageRole::Assistant {
				format!(
					"<post author={} author_kind={} author_id={}>{}</post>",
					post.actor.name(),
					post.actor.kind().input_str(),
					post.actor.id(),
					value
				)
			} else {
				value.to_string()
			};
			InputItem::Message(MessageParam {
				id: None,
				role,
				content: MessageContent::Text(text),
				status: None,
			})
		}
	};
	input_item.xok()
}



pub fn ev_to_response_partial(
	prev: Option<ResponsePartial>,
	ev: StreamingEvent,
) -> Result<ResponsePartial> {
	use StreamingEvent::*;
	match ev {
		ResponseCreated(ResponseCreatedEvent { response, .. })
		| ResponseQueued(ResponseQueuedEvent { response, .. })
		| ResponseInProgress(ResponseInProgressEvent { response, .. })
		| ResponseIncomplete(ResponseIncompleteEvent { response, .. })
		| ResponseCompleted(ResponseCompletedEvent { response, .. })
		| ResponseFailed(ResponseFailedEvent { response, .. }) => {
			response_to_partial(response)
		}
		ev => {
			let mut partial = prev.ok_or_else(|| {
				bevyhow!("Received non-response event with no previous state")
			})?;
			partial.posts = stream_to_partial_posts(ev)?;

			partial.xok()
		}
	}
}

/// Create a [`ResponsePartial`] from a [`ResponseBody`], including any output items.
pub fn response_to_partial(response: ResponseBody) -> Result<ResponsePartial> {
	use o11s::response::Status::*;
	let post_status = match response.status {
		Completed => PostStatus::Completed,
		InProgress | Queued => PostStatus::InProgress,
		Incomplete | Failed | Cancelled => PostStatus::Interrupted,
	};
	ResponsePartial {
		posts: PostPartial::from_output_items(response.output, post_status)
			.into_iter()
			.collect(),
		response_id: response.id,
		response_stored: response.store.unwrap_or(false),
		status: match response.status {
			InProgress => ResponseStatus::InProgress,
			Completed => ResponseStatus::Completed,
			Incomplete => ResponseStatus::Incomplete(
				response.incomplete_details.map(|d| d.reason),
			),
			Failed => match response.error {
				Some(err) => ResponseStatus::Failed {
					code: Some(err.code),
					message: Some(err.message),
				},
				None => ResponseStatus::Failed {
					code: None,
					message: None,
				},
			},
			Cancelled => ResponseStatus::Cancelled,
			Queued => ResponseStatus::Queued,
		},
		token_usage: response.usage.map(|usage| TokenUsage {
			input_tokens: usage.input_tokens,
			output_tokens: usage.output_tokens,
			total_tokens: usage.total_tokens,
			cached_input_tokens: usage
				.input_tokens_details
				.map(|d| d.cached_tokens),
			reasoning_tokens: usage
				.output_tokens_details
				.map(|d| d.reasoning_tokens),
		}),
	}
	.xok()
}


/// ## Errors
/// Errors if a [`StreamingEvent::Error`] is received
fn stream_to_partial_posts(event: StreamingEvent) -> Result<Vec<PostPartial>> {
	use StreamingEvent::*;
	// trace!("Streaming Event: {:#?}", event);
	match event {
		ResponseCreated(ev) => {
			// usually empty but parse anyway
			PostPartial::from_output_items(
				ev.response.output,
				PostStatus::InProgress,
			)
			.into_iter()
			.collect::<Vec<_>>()
		}
		ResponseQueued(ev) => {
			// usually empty but parse anyway
			PostPartial::from_output_items(
				ev.response.output,
				PostStatus::InProgress,
			)
			.into_iter()
			.collect::<Vec<_>>()
		}
		ResponseInProgress(ev) => {
			// usually empty but parse anyway
			PostPartial::from_output_items(
				ev.response.output,
				PostStatus::InProgress,
			)
			.into_iter()
			.collect::<Vec<_>>()
		}
		ResponseCompleted(ev) => {
			// usually empty but parse anyway
			PostPartial::from_output_items(
				ev.response.output,
				PostStatus::Completed,
			)
			.into_iter()
			.collect::<Vec<_>>()
		}
		ResponseFailed(ev) => {
			// usually empty but parse anyway
			PostPartial::from_output_items(
				ev.response.output,
				PostStatus::Interrupted,
			)
			.into_iter()
			.collect::<Vec<_>>()
		}
		ResponseIncomplete(ev) => {
			// usually empty but parse anyway
			PostPartial::from_output_items(
				ev.response.output,
				PostStatus::Interrupted,
			)
			.into_iter()
			.collect::<Vec<_>>()
		}
		OutputItemAdded(item_added) => PostPartial::from_output_items(
			item_added.item,
			PostStatus::InProgress,
		)
		.into_iter()
		.collect::<Vec<_>>(),
		OutputItemDone(item_done) => PostPartial::from_output_items(
			item_done.item,
			PostStatus::Completed,
		)
		.into_iter()
		.collect::<Vec<_>>(),
		ContentPartAdded(part_added) => PostPartial {
			key: PostPartialKey::Content {
				responses_id: part_added.item_id,
				content_index: part_added.content_index,
			},
			status: PostStatus::InProgress,
			content: PartialContent::ContentPart(part_added.part),
		}
		.xvec(),
		ContentPartDone(part_done) => PostPartial {
			key: PostPartialKey::Content {
				responses_id: part_done.item_id,
				content_index: part_done.content_index,
			},
			status: PostStatus::Completed,
			content: PartialContent::ContentPart(part_done.part),
		}
		.xvec(),
		OutputTextDelta(text_delta) => PostPartial {
			key: PostPartialKey::Content {
				responses_id: text_delta.item_id,
				content_index: text_delta.content_index,
			},
			status: PostStatus::InProgress,
			content: PartialContent::Delta(text_delta.delta),
		}
		.xvec(),
		OutputTextDone(text_done) => PostPartial {
			key: PostPartialKey::Content {
				responses_id: text_done.item_id,
				content_index: text_done.content_index,
			},
			status: PostStatus::Completed,
			content: PartialContent::TextDone {
				text: text_done.text,
				logprobs: text_done.logprobs,
			},
		}
		.xvec(),
		OutputTextAnnotationAdded(annotation_added) => {
			if let Some(annotation) = annotation_added.annotation {
				PostPartial {
					key: PostPartialKey::Content {
						responses_id: annotation_added.item_id,
						content_index: annotation_added.content_index,
					},
					status: PostStatus::InProgress,
					content: PartialContent::AnnotationAdded {
						annotation,
						annotation_index: annotation_added.annotation_index,
					},
				}
				.xvec()
			} else {
				vec![]
				// no annotation, nothing to do
			}
		}
		RefusalDelta(refusal_delta) => PostPartial {
			key: PostPartialKey::Content {
				responses_id: refusal_delta.item_id,
				content_index: refusal_delta.content_index,
			},
			status: PostStatus::InProgress,
			content: PartialContent::Delta(refusal_delta.delta),
		}
		.xvec(),
		RefusalDone(refusal_done) => PostPartial {
			key: PostPartialKey::Content {
				responses_id: refusal_done.item_id,
				content_index: refusal_done.content_index,
			},
			status: PostStatus::Completed,
			content: PartialContent::RefusalDone {
				refusal: refusal_done.refusal,
			},
		}
		.xvec(),
		ReasoningDelta(reasoning_delta) => PostPartial {
			key: PostPartialKey::Content {
				responses_id: reasoning_delta.item_id,
				content_index: reasoning_delta.content_index,
			},
			status: PostStatus::InProgress,
			content: PartialContent::Delta(reasoning_delta.delta),
		}
		.xvec(),
		ReasoningDone(reasoning_done) => PostPartial {
			key: PostPartialKey::Content {
				responses_id: reasoning_done.item_id,
				content_index: reasoning_done.content_index,
			},
			status: PostStatus::Completed,
			content: PartialContent::ReasoningDone {
				content: reasoning_done.text,
			},
		}
		.xvec(),
		ReasoningSummaryTextDelta(summary_delta) => PostPartial {
			key: PostPartialKey::ReasoningSummary {
				responses_id: summary_delta.item_id,
				summary_index: summary_delta.summary_index.unwrap_or(0),
			},
			status: PostStatus::InProgress,
			content: PartialContent::Delta(summary_delta.delta),
		}
		.xvec(),
		ReasoningSummaryTextDone(summary_done) => PostPartial {
			key: PostPartialKey::ReasoningSummary {
				responses_id: summary_done.item_id,
				summary_index: summary_done.summary_index.unwrap_or(0),
			},
			status: PostStatus::Completed,
			content: PartialContent::ReasoningSummary(summary_done.text),
		}
		.xvec(),
		ReasoningSummaryPartAdded(summary_added) => PostPartial {
			key: PostPartialKey::ReasoningSummary {
				responses_id: summary_added.item_id,
				summary_index: summary_added.summary_index.unwrap_or(0),
			},
			status: PostStatus::InProgress,
			content: PartialContent::ContentPart(summary_added.part),
		}
		.xvec(),
		ReasoningSummaryPartDone(summary_done) => PostPartial {
			key: PostPartialKey::ReasoningSummary {
				responses_id: summary_done.item_id,
				summary_index: summary_done.summary_index.unwrap_or(0),
			},
			status: PostStatus::Completed,
			content: PartialContent::ContentPart(summary_done.part),
		}
		.xvec(),
		FunctionCallArgumentsDelta(arguments_delta) => PostPartial {
			key: PostPartialKey::Single {
				responses_id: arguments_delta.item_id,
			},
			status: PostStatus::InProgress,
			content: PartialContent::Delta(arguments_delta.delta),
		}
		.xvec(),
		FunctionCallArgumentsDone(arguments_done) => PostPartial {
			key: PostPartialKey::Single {
				responses_id: arguments_done.item_id,
			},
			status: PostStatus::Completed,
			content: PartialContent::FunctionCallArgumentsDone(
				arguments_done.arguments,
			),
		}
		.xvec(),
		Error(error) => {
			bevybail!("Model streaming error: {:?}", error.error);
		}
	}
	.xok()
}
