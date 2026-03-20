use crate::openresponses::ContentPart;
use crate::openresponses::MessageRole;
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
	meta: Option<O11sMeta>,
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
				call_id: get_call_id(&meta)?,
				name: function_call.function_name().to_string(),
				arguments: function_call.args().to_string(),
				status: None,
			})
		}
		ActionPayload::FunctionCallOutput(output_item) => {
			InputItem::FunctionCallOutput(FunctionCallOutputParam {
				id: None,
				// NOTE: in the case of a function call output without an O11sMeta,
				// this will actually be the meta of the FunctionCall, we do that
				// to get the correct call id.
				call_id: get_call_id(&meta)?,
				output: FunctionOutputContent::Text(
					output_item.output().to_string(),
				),
				status: None,
			})
		}
	};
	input_item.xok()
}

fn get_call_id(meta: &Option<O11sMeta>) -> Result<String> {
	meta.as_ref()
		.ok_or_else(|| bevyhow!("O11sMeta missing for function call"))?
		.call_id()
		.map(|s| s.to_string())
		.ok_or_else(|| bevyhow!("O11sMeta has no call_id"))
}
