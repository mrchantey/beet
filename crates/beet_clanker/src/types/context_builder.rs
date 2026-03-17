use crate::openresponses::ContentPart;
use crate::openresponses::InputFile;
use crate::openresponses::MessageStatus;
use crate::openresponses::OutputContent;
use crate::openresponses::OutputItem;
use crate::openresponses::request::FunctionCallOutputParam;
use crate::openresponses::request::FunctionCallParam;
use crate::openresponses::request::FunctionOutputContent;
use crate::openresponses::request::Input;
use crate::openresponses::request::InputItem;
use crate::openresponses::request::MessageContent;
use crate::openresponses::request::MessageParam;
use crate::prelude::*;
use beet_core::prelude::*;

#[derive(Debug, Default, Clone)]
pub struct ContextBuilder {}


impl ContextBuilder {
	pub fn new() -> Self { Self {} }
	pub fn build_input(
		&self,
		map: &ContextMap,
		agent_id: ActorId,
		thread_id: ThreadId,
		last_sent_item: Option<ItemId>,
	) -> Result<openresponses::request::Input> {
		let thread = map.threads().get(thread_id)?;

		let items = if let Some(last_sent_item) = last_sent_item {
			thread.items_after(last_sent_item)
		} else {
			thread.items()
		};

		// threads are strictly already chronologically sorted by uuidv7,
		// no need to sort here.
		let items = items.into_iter().xtry_map(|item_id| {
			self.into_openresponses_input(map, agent_id, *item_id)
		})?;

		Input::Items(items).xok()
	}

	/// Map an item to a list of openresponses input, relative to agiven actor.
	/// The provided actor is used to correctly assign a [`MessageRole::Assistant`]
	/// for 'self' messages, and [`MessageRole::User`] for all others.
	///
	/// this may be several items, for example a [`Item::FunctionCall`]
	/// is split into an openresponses FunctionCall + FunctionCallOutput,
	/// assigned a call_id on the fly.
	fn into_openresponses_input(
		&self,
		map: &ContextMap,
		agent_id: ActorId,
		item_id: ItemId,
	) -> Result<openresponses::request::InputItem> {
		let item = map.items().get(item_id)?;
		let owner = map.actors().get(item.owner())?;
		let role = item_message_role(agent_id, owner);

		let item = match item.content() {
			Content::Text(TextItem(value)) => {
				InputItem::Message(MessageParam {
					id: None,
					role,
					// TODO prefix with owner name, ie billy says: >
					content: MessageContent::Text(value.clone()),
					status: None,
				})
			}
			Content::Refusal(RefusalItem(value)) => {
				InputItem::Message(MessageParam {
					id: None,
					role,
					content: MessageContent::Text(value.clone()),
					status: None,
				})
			}
			Content::ReasoningSummary(ReasoningSummaryItem(value)) => {
				InputItem::Message(MessageParam {
					id: None,
					role,
					content: MessageContent::Text(value.clone()),
					status: None,
				})
			}
			Content::ReasoningContent(ReasoningContentItem(value)) => {
				InputItem::Message(MessageParam {
					id: None,
					role,
					content: MessageContent::Text(value.clone()),
					status: None,
				})
			}
			Content::ReasoningEncryptedContent(
				ReasoningEncryptedContentItem(value),
			) => InputItem::Message(MessageParam {
				id: None,
				role,
				content: MessageContent::Text(value.clone()),
				status: None,
			}),
			Content::Url(url_item) => {
				InputItem::Message(MessageParam {
					id: None,
					role,
					content: MessageContent::Parts(vec![
						ContentPart::InputFile(InputFile {
							filename: Some(url_item.filename()),
							// TODO distinguish base64 encoded urls..
							file_data: None,
							file_url: Some(url_item.url().to_string()),
						}),
					]),
					status: None,
				})
			}
			Content::Bytes(bytes_item) => InputItem::Message(MessageParam {
				id: None,
				role,
				content: MessageContent::Parts(vec![ContentPart::InputFile(
					InputFile {
						filename: Some(bytes_item.filename()),
						file_data: Some(bytes_item.bytes_base64()),
						file_url: None,
					},
				)]),
				status: None,
			}),
			Content::FunctionCall(function_call) => {
				InputItem::FunctionCall(FunctionCallParam {
					id: None,
					// call_id is the function call item id
					call_id: item.id().to_string(),
					name: function_call.function_name().to_string(),
					arguments: function_call.args().to_string(),
					status: None,
				})
			}
			Content::FunctionCallOutput(output_item) => {
				InputItem::FunctionCallOutput(FunctionCallOutputParam {
					id: None,
					call_id: output_item.function_call_item.to_string(),
					output: FunctionOutputContent::Text(
						output_item.output().to_string(),
					),
					status: None,
				})
			}
		};
		item.xok()
	}

	pub fn parse_output(
		&self,
		context_query: &mut ContextQuery,
		owner: ActorId,
		items: Vec<OutputItem>,
	) -> Result<()> {
		let items = items
			.into_iter()
			.xtry_map(|item| self.from_openresponses_output(owner, item))?
			.into_iter()
			.flatten();

		context_query.add_items(items)?;

		Ok(())
	}

	fn from_openresponses_output(
		&self,
		owner: ActorId,
		item: OutputItem,
	) -> Result<Vec<Item>> {
		let mut out = Vec::new();
		// Item::new(actor_id, content)
		match item {
			OutputItem::Message(message) => {
				let status = match message.status {
					MessageStatus::InProgress => ItemStatus::InProgress,
					MessageStatus::Completed => ItemStatus::Completed,
					MessageStatus::Incomplete => ItemStatus::Interrupted,
				};
				for content in message.content.into_iter() {
					match content {
						OutputContent::OutputText(output_text) => {
							if !output_text.annotations.is_empty() {
								todo!("inline annotations as markdown links");
							}
							out.push(Item::new(
								owner,
								status,
								TextItem(output_text.text),
							));
						}
						OutputContent::Refusal(refusal) => {
							out.push(Item::new(
								owner,
								status,
								RefusalItem(refusal.refusal),
							));
						}
					}
				}
			}
			OutputItem::FunctionCall(_function_call) => {
				todo!("incomplete function call");
			}
			OutputItem::FunctionCallOutput(_function_call_output_item) => {
				todo!("find incomplete function call and match with output")
			}
			OutputItem::Reasoning(reasoning_item) => {
				let summary = reasoning_item.all_summary();
				if !summary.is_empty() {
					out.push(Item::new(
						owner,
						ItemStatus::Completed,
						ReasoningSummaryItem(summary),
					));
				}
				let content = reasoning_item.all_content();
				if !content.is_empty() {
					out.push(Item::new(
						owner,
						ItemStatus::Completed,
						ReasoningContentItem(content),
					));
				}
				let encrypted_content = reasoning_item.all_encrypted_content();
				if !encrypted_content.is_empty() {
					out.push(Item::new(
						owner,
						ItemStatus::Completed,
						ReasoningEncryptedContentItem(encrypted_content),
					));
				}
			}
		}
		out.xok()
	}
}

/// Get the message role for this actor, relative to the items actor id.
/// This is useful when an agent is constructing its context for an
/// openresponses request.
fn item_message_role(
	agent_id: ActorId,
	owner: &Actor,
) -> openresponses::MessageRole {
	use openresponses::MessageRole;
	match owner.kind() {
		ActorKind::System => MessageRole::System,
		ActorKind::Developer => MessageRole::Developer,
		ActorKind::Human => MessageRole::User,
		ActorKind::Agent => {
			if owner.id() == agent_id {
				MessageRole::Assistant
			} else {
				MessageRole::User
			}
		}
	}
}
