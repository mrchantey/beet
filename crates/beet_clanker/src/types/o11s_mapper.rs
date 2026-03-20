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
