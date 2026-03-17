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
use std::sync::atomic::AtomicU64;

#[derive(Debug, Default, Clone)]
pub struct ContextBuilder {}




impl ContextBuilder {
	pub fn new() -> Self { Self {} }
	pub fn build_input(
		&self,
		map: &ContextMap,
		actor_id: ActorId,
		last_sent_item: Option<ItemId>,
	) -> Result<openresponses::request::Input> {
		let actor = map.actor(actor_id)?;

		let items = if let Some(last_sent_item) = last_sent_item {
			actor.items_after(last_sent_item)
		} else {
			actor.items()
		};

		let mut timestamped_items = items
			.iter()
			.xtry_map(|item_id| self.item_to_input(map, &actor, *item_id))?;

		timestamped_items.sort_by_key(|(timestamp, _)| *timestamp);

		let items = timestamped_items
			.into_iter()
			.map(|(_, items)| items)
			.flatten()
			.collect::<Vec<_>>();

		Input::Items(items).xok()
	}

	/// Map an item to a list of openresponses input, relative to agiven actor.
	/// The provided actor is used to correctly assign a [`MessageRole::Assistant`]
	/// for 'self' messages, and [`MessageRole::User`] for all others.
	///
	/// this may be several items, for example a [`Item::FunctionCall`]
	/// is split into an openresponses FunctionCall + FunctionCallOutput,
	/// assigned a call_id on the fly.
	pub fn item_to_input(
		&self,
		map: &ContextMap,
		agent: &Actor,
		item_id: ItemId,
	) -> Result<(Timestamp, Vec<openresponses::request::InputItem>)> {
		let item = map.item(item_id)?;
		let owner = map.actor(item.owner())?;
		let role = item_message_role(agent, owner);

		let items = match item.content() {
			Content::Text(text_content) => {
				vec![InputItem::Message(MessageParam {
					id: None,
					role,
					content: MessageContent::Text(
						text_content.content().to_string(),
					),
					status: None,
				})]
			}
			Content::File(file_content) => {
				vec![InputItem::Message(MessageParam {
					id: None,
					role,
					content: MessageContent::Parts(vec![
						ContentPart::InputFile(InputFile {
							filename: Some(file_content.filename()),
							// TODO distinguish base64 encoded urls..
							file_data: None,
							file_url: Some(file_content.url().to_string()),
						}),
					]),
					status: None,
				})]
			}
			Content::FunctionCall(function_call) => {
				static CALL_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

				let call_id = CALL_ID_COUNTER
					.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
					.to_string();

				vec![
					InputItem::FunctionCall(FunctionCallParam {
						id: None,
						call_id: call_id.clone(),
						name: function_call.function_name().to_string(),
						arguments: function_call.args().to_string(),
						status: None,
					}),
					InputItem::FunctionCallOutput(FunctionCallOutputParam {
						id: None,
						call_id,
						output: FunctionOutputContent::Text(
							function_call.output().to_string(),
						),
						status: None,
					}),
				]
			}
		};
		(item.created(), items).xok()
	}

	pub fn parse_output(
		&self,
		context_query: &mut ContextQuery,
		actor_id: ActorId,
		items: Vec<OutputItem>,
	) -> Result<()> {
		let scope = ItemScope::Family;
		let items = items
			.into_iter()
			.xtry_map(|item| self.output_item_to_content(item))?
			.into_iter()
			.flatten()
			.map(|content| Item::new(actor_id, content, scope.clone()));

		context_query.add_items(items)?;

		Ok(())
	}

	fn output_item_to_content(&self, item: OutputItem) -> Result<Vec<Content>> {
		match item {
			OutputItem::Message(message)
				if message.status == MessageStatus::Completed =>
			{
				message
					.content
					.into_iter()
					.map(|content| match content {
						OutputContent::OutputText(output_text) => {
							if !output_text.annotations.is_empty() {
								todo!("inline annotations as markdown links");
							}
							TextContent::message(output_text.text).into()
						}
						OutputContent::Refusal(refusal) => {
							TextContent::refusal(refusal.refusal).into()
						}
					})
					.collect::<Vec<_>>()
					.xok()
			}
			OutputItem::Message(_message) => {
				todo!("incomplete message");
			}
			OutputItem::FunctionCall(_function_call) => {
				todo!("incomplete function call");
			}
			OutputItem::FunctionCallOutput(_function_call_output_item) => {
				todo!("find incomplete function call and match with output")
			}
			OutputItem::Reasoning(reasoning_item) => {
				let mut out = Vec::new();
				let summary = reasoning_item.all_summary();
				if !summary.is_empty() {
					out.push(Content::Text(TextContent::reasoning_summary(
						summary,
					)));
				}
				let content = reasoning_item.all_content();
				if !content.is_empty() {
					out.push(Content::Text(TextContent::reasoning_content(
						content,
					)));
				}
				let encrypted_content = reasoning_item.all_encrypted_content();
				if !encrypted_content.is_empty() {
					out.push(Content::Text(
						TextContent::reasoning_encrypted_content(
							encrypted_content,
						),
					));
				}
				out.xok()
			}
		}
	}
}

/// Get the message role for this actor, relative to the items actor id.
/// This is useful when an agent is constructing its context for an
/// openresponses request.
fn item_message_role(
	agent: &Actor,
	owner: &Actor,
) -> openresponses::MessageRole {
	use openresponses::MessageRole;
	match owner.kind() {
		ActorKind::System => MessageRole::System,
		ActorKind::Developer => MessageRole::Developer,
		ActorKind::Human => MessageRole::User,
		ActorKind::Agent => {
			if owner.id() == agent.id() {
				MessageRole::Assistant
			} else {
				MessageRole::User
			}
		}
	}
}
