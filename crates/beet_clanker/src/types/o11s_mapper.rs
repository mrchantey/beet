use crate::openresponses::ContentPart;
use crate::openresponses::MessageRole;
use crate::openresponses::ResponseBody;
use crate::openresponses::StreamingEvent;
use crate::openresponses::request::FunctionCallOutputParam;
use crate::openresponses::request::FunctionCallParam;
use crate::openresponses::request::FunctionOutputContent;
use crate::openresponses::request::InputItem;
use crate::openresponses::request::MessageContent;
use crate::openresponses::request::MessageParam;
use crate::openresponses::streaming::ResponseCompletedEvent;
use crate::openresponses::streaming::ResponseCreatedEvent;
use crate::openresponses::streaming::ResponseFailedEvent;
use crate::openresponses::streaming::ResponseInProgressEvent;
use crate::openresponses::streaming::ResponseIncompleteEvent;
use crate::openresponses::streaming::ResponseQueuedEvent;
use crate::prelude::*;
use beet_core::prelude::*;


pub fn action_to_o11s_input(
	agent_id: ActorId,
	action: Action,
	author: Actor,
	meta: ActionMeta,
) -> Result<openresponses::request::InputItem> {
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
			let actor_text = format!(
				"<actor name={} kind={} id={}>{}</actor>",
				author.name(),
				author.kind().input_str(),
				author.id(),
				value
			);
			InputItem::Message(MessageParam {
				id: None,
				role,
				content: MessageContent::Text(actor_text),
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
		ActionPayload::Bytes(bytes_item) => InputItem::Message(MessageParam {
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
		ActionPayload::FunctionCall(function_call) => {
			InputItem::FunctionCall(FunctionCallParam {
				id: None,
				call_id: meta
					.call_id()
					.ok_or_else(|| bevyhow!("ActionMeta has no call_id"))?
					.to_string(),
				name: function_call.function_name().to_string(),
				arguments: function_call.args().to_string(),
				status: None,
			})
		}
		ActionPayload::FunctionCallOutput(output_item) => {
			InputItem::FunctionCallOutput(FunctionCallOutputParam {
				id: None,
				call_id: meta
					.call_id()
					.ok_or_else(|| bevyhow!("ActionMeta has no call_id"))?
					.to_string(),
				output: FunctionOutputContent::Text(
					output_item.output().to_string(),
				),
				status: None,
			})
		}
	};
	input_item.xok()
}


/// ## Errors
/// Errors if a [`StreamingEvent::Error`] is received
pub fn o11s_stream_to_partial_actions(
	event: StreamingEvent,
) -> Result<Vec<ActionPartial>> {
	use StreamingEvent::*;
	// trace!("Streaming Event: {:#?}", event);
	match event {
		ResponseCreated(ev) => {
			// usually empty but parse anyway
			ActionPartial::from_output_items(
				ev.response.output,
				ActionStatus::InProgress,
			)
			.into_iter()
			.collect::<Vec<_>>()
		}
		ResponseQueued(ev) => {
			// usually empty but parse anyway
			ActionPartial::from_output_items(
				ev.response.output,
				ActionStatus::InProgress,
			)
			.into_iter()
			.collect::<Vec<_>>()
		}
		ResponseInProgress(ev) => {
			// usually empty but parse anyway
			ActionPartial::from_output_items(
				ev.response.output,
				ActionStatus::InProgress,
			)
			.into_iter()
			.collect::<Vec<_>>()
		}
		ResponseCompleted(ev) => {
			// usually empty but parse anyway
			ActionPartial::from_output_items(
				ev.response.output,
				ActionStatus::Completed,
			)
			.into_iter()
			.collect::<Vec<_>>()
		}
		ResponseFailed(ev) => {
			// usually empty but parse anyway
			ActionPartial::from_output_items(
				ev.response.output,
				ActionStatus::Interrupted,
			)
			.into_iter()
			.collect::<Vec<_>>()
		}
		ResponseIncomplete(ev) => {
			// usually empty but parse anyway
			ActionPartial::from_output_items(
				ev.response.output,
				ActionStatus::Interrupted,
			)
			.into_iter()
			.collect::<Vec<_>>()
		}
		OutputItemAdded(item_added) => ActionPartial::from_output_items(
			item_added.item,
			ActionStatus::InProgress,
		)
		.into_iter()
		.collect::<Vec<_>>(),
		OutputItemDone(item_done) => ActionPartial::from_output_items(
			item_done.item,
			ActionStatus::Completed,
		)
		.into_iter()
		.collect::<Vec<_>>(),
		ContentPartAdded(part_added) => ActionPartial {
			key: ActionPartialKey::Content {
				responses_id: part_added.item_id,
				content_index: part_added.content_index,
			},
			status: ActionStatus::InProgress,
			content: PartialContent::ContentPart(part_added.part),
		}
		.xvec(),
		ContentPartDone(part_done) => ActionPartial {
			key: ActionPartialKey::Content {
				responses_id: part_done.item_id,
				content_index: part_done.content_index,
			},
			status: ActionStatus::Completed,
			content: PartialContent::ContentPart(part_done.part),
		}
		.xvec(),
		OutputTextDelta(text_delta) => ActionPartial {
			key: ActionPartialKey::Content {
				responses_id: text_delta.item_id,
				content_index: text_delta.content_index,
			},
			status: ActionStatus::InProgress,
			content: PartialContent::Delta(text_delta.delta),
		}
		.xvec(),
		OutputTextDone(text_done) => ActionPartial {
			key: ActionPartialKey::Content {
				responses_id: text_done.item_id,
				content_index: text_done.content_index,
			},
			status: ActionStatus::Completed,
			content: PartialContent::TextDone {
				text: text_done.text,
				logprobs: text_done.logprobs,
			},
		}
		.xvec(),
		OutputTextAnnotationAdded(annotation_added) => {
			if let Some(annotation) = annotation_added.annotation {
				ActionPartial {
					key: ActionPartialKey::Content {
						responses_id: annotation_added.item_id,
						content_index: annotation_added.content_index,
					},
					status: ActionStatus::InProgress,
					content: PartialContent::AnnotationAdded {
						annotation,
						annotation_index: annotation_added.annotation_index,
					},
				}
				.xvec()
			} else {
				default()
				// no annotation, nothing to do
			}
		}
		RefusalDelta(refusal_delta) => ActionPartial {
			key: ActionPartialKey::Content {
				responses_id: refusal_delta.item_id,
				content_index: refusal_delta.content_index,
			},
			status: ActionStatus::InProgress,
			content: PartialContent::Delta(refusal_delta.delta),
		}
		.xvec(),
		RefusalDone(refusal_done) => ActionPartial {
			key: ActionPartialKey::Content {
				responses_id: refusal_done.item_id,
				content_index: refusal_done.content_index,
			},
			status: ActionStatus::Completed,
			content: PartialContent::RefusalDone {
				refusal: refusal_done.refusal,
			},
		}
		.xvec(),
		ReasoningDelta(reasoning_delta) => ActionPartial {
			key: ActionPartialKey::Content {
				responses_id: reasoning_delta.item_id,
				content_index: reasoning_delta.content_index,
			},
			status: ActionStatus::InProgress,
			content: PartialContent::Delta(reasoning_delta.delta),
		}
		.xvec(),
		ReasoningDone(reasoning_done) => ActionPartial {
			key: ActionPartialKey::Content {
				responses_id: reasoning_done.item_id,
				content_index: reasoning_done.content_index,
			},
			status: ActionStatus::Completed,
			content: PartialContent::ReasoningDone {
				content: reasoning_done.text,
			},
		}
		.xvec(),
		ReasoningSummaryTextDelta(summary_delta) => ActionPartial {
			key: ActionPartialKey::ReasoningSummary {
				responses_id: summary_delta.item_id,
				summary_index: summary_delta.summary_index.unwrap_or(0),
			},
			status: ActionStatus::InProgress,
			content: PartialContent::Delta(summary_delta.delta),
		}
		.xvec(),
		ReasoningSummaryTextDone(summary_done) => ActionPartial {
			key: ActionPartialKey::ReasoningSummary {
				responses_id: summary_done.item_id,
				summary_index: summary_done.summary_index.unwrap_or(0),
			},
			status: ActionStatus::Completed,
			content: PartialContent::ReasoningSummary(summary_done.text),
		}
		.xvec(),
		ReasoningSummaryPartAdded(summary_added) => ActionPartial {
			key: ActionPartialKey::ReasoningSummary {
				responses_id: summary_added.item_id,
				summary_index: summary_added.summary_index.unwrap_or(0),
			},
			status: ActionStatus::InProgress,
			content: PartialContent::ContentPart(summary_added.part),
		}
		.xvec(),
		ReasoningSummaryPartDone(summary_done) => ActionPartial {
			key: ActionPartialKey::ReasoningSummary {
				responses_id: summary_done.item_id,
				summary_index: summary_done.summary_index.unwrap_or(0),
			},
			status: ActionStatus::Completed,
			content: PartialContent::ContentPart(summary_done.part),
		}
		.xvec(),
		FunctionCallArgumentsDelta(arguments_delta) => ActionPartial {
			key: ActionPartialKey::Single {
				responses_id: arguments_delta.item_id,
			},
			status: ActionStatus::InProgress,
			content: PartialContent::Delta(arguments_delta.delta),
		}
		.xvec(),
		FunctionCallArgumentsDone(arguments_done) => ActionPartial {
			key: ActionPartialKey::Single {
				responses_id: arguments_done.item_id,
			},
			status: ActionStatus::Completed,
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



pub fn ev_to_stream_state(
	prev: Option<ResponsePartial>,
	ev: &StreamingEvent,
) -> Result<ResponsePartial> {
	use StreamingEvent::*;
	match ev {
		ResponseCreated(ResponseCreatedEvent { response, .. })
		| ResponseQueued(ResponseQueuedEvent { response, .. })
		| ResponseInProgress(ResponseInProgressEvent { response, .. })
		| ResponseIncomplete(ResponseIncompleteEvent { response, .. })
		| ResponseCompleted(ResponseCompletedEvent { response, .. })
		| ResponseFailed(ResponseFailedEvent { response, .. }) => {
			response_to_stream_state(response)
		}
		Error(err) => {
			bevybail!("Received error event in stream: {:?}", err.error);
		}
		_ => prev.ok_or_else(|| {
			bevyhow!("Received non-response event with no previous state")
		}),
	}
}

/// Create a stream state with no actions
fn response_to_stream_state(
	response: &ResponseBody,
) -> Result<ResponsePartial> {
	ResponsePartial {
		actions: default(),
		response_id: response.id.clone(),
		response_stored: response.store.unwrap_or(false),
		status: {
			use openresponses::response::Status::*;
			match response.status {
				InProgress => ActionStreamStatus::InProgress,
				Completed => ActionStreamStatus::Completed,
				Incomplete => ActionStreamStatus::Incomplete(
					response.incomplete_details.clone().map(|d| d.reason),
				),
				Failed => match response.error.clone() {
					Some(err) => ActionStreamStatus::Failed {
						code: Some(err.code),
						message: Some(err.message),
					},
					None => ActionStreamStatus::Failed {
						code: None,
						message: None,
					},
				},
				Cancelled => ActionStreamStatus::Cancelled,
				Queued => ActionStreamStatus::Queued,
			}
		},
		token_usage: response.usage.clone().map(|usage| TokenUsage {
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
