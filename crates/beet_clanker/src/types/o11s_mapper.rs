use crate::openresponses::ContentPart;
use crate::openresponses::MessageRole;
use crate::openresponses::OutputItem;
use crate::openresponses::ResponseBody;
use crate::openresponses::StreamingEvent;
use crate::openresponses::request::FunctionCallOutputParam;
use crate::openresponses::request::FunctionCallParam;
use crate::openresponses::request::FunctionOutputContent;
use crate::openresponses::request::InputItem;
use crate::openresponses::request::MessageContent;
use crate::openresponses::request::MessageParam;
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



pub fn o11s_stream_event_to_output(
	action_store: impl ActionStore,
	prev_state: Option<ActionStreamState>,
	ev: StreamingEvent,
	// agent_id: ActorId,
	// action: Action,
	// author: Actor,
	// meta: ActionMeta,
) -> Result<ActionStreamState> {
	use StreamingEvent::*;
	match ev {
		ResponseCreated(ev) => {
			response_to_stream_state(action_store, ev.response)
		}
		ResponseQueued(ev) => {
			response_to_stream_state(action_store, ev.response)
		}
		ResponseInProgress(ev) => {
			response_to_stream_state(action_store, ev.response)
		}
		ResponseCompleted(ev) => {
			response_to_stream_state(action_store, ev.response)
		}
		ResponseFailed(ev) => {
			response_to_stream_state(action_store, ev.response)
		}
		ResponseIncomplete(ev) => {
			response_to_stream_state(action_store, ev.response)
		}
		OutputItemAdded(output_item_added_event) => todo!(),
		OutputItemDone(output_item_done_event) => todo!(),
		ContentPartAdded(content_part_added_event) => todo!(),
		ContentPartDone(content_part_done_event) => todo!(),
		OutputTextDelta(output_text_delta_event) => todo!(),
		OutputTextDone(output_text_done_event) => todo!(),
		OutputTextAnnotationAdded(output_text_annotation_added_event) => {
			todo!()
		}
		RefusalDelta(refusal_delta_event) => todo!(),
		RefusalDone(refusal_done_event) => todo!(),
		ReasoningDelta(reasoning_delta_event) => todo!(),
		ReasoningDone(reasoning_done_event) => todo!(),
		ReasoningSummaryTextDelta(reasoning_summary_text_delta_event) => {
			todo!()
		}
		ReasoningSummaryTextDone(reasoning_summary_text_done_event) => todo!(),
		ReasoningSummaryPartAdded(reasoning_summary_part_added_event) => {
			todo!()
		}
		ReasoningSummaryPartDone(reasoning_summary_part_done_event) => todo!(),
		FunctionCallArgumentsDelta(function_call_arguments_delta_event) => {
			todo!()
		}
		FunctionCallArgumentsDone(function_call_arguments_done_event) => {
			todo!()
		}
		Error(error_event) => todo!(),
	}
}


fn response_to_stream_state(
	action_store: impl ActionStore,
	response: ResponseBody,
) -> Result<ActionStreamState> {
	ActionStreamState {
		mutations: output_items_to_mutations(
			action_store,
			&response.id,
			response.store.unwrap_or(false),
			response.output,
		)?,
		response_id: response.id,
		response_stored: response.store.unwrap_or(false),
		status: {
			use openresponses::response::Status::*;
			match response.status {
				InProgress => ActionStreamStatus::InProgress,
				Completed => ActionStreamStatus::Completed,
				Incomplete => ActionStreamStatus::Incomplete(
					response.incomplete_details.map(|d| d.reason),
				),
				Failed => match response.error {
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


fn require_prev_state(
	prev_state: Option<ActionStreamState>,
) -> Result<ActionStreamState> {
	prev_state.ok_or_else(|| {
		bevyhow!(
			"Stream Order Error: Previous state is required for partial streams"
		)
	})
}


fn output_items_to_mutations(
	action_store: impl ActionStore,
	response_id: &str,
	respose_stored: bool,
	items: Vec<OutputItem>,
) -> Result<HashMap<ActionId, ActionMutation>> {
	let mut map = HashMap::default();
	/// Only insert if no existing key or it is a lower level,
	/// ie Updated does not clobber Created.
	fn selective_insert(
		map: &mut HashMap<ActionId, ActionMutation>,
		id: ActionId,
		mutation: ActionMutation,
	) {
		use ActionMutation::*;
		match mutation {
			Created => {
				map.entry(id).or_insert(Created);
			}
			Updated => {
				map.entry(id).or_insert(Updated);
			}
		}
	}
	for item in items {
		todo!()
	}

	Ok(map)
}
