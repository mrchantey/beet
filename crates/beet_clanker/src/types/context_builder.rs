use crate::openresponses::ContentPart;
use crate::openresponses::InputFile;
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
	pub fn build(
		&self,
		query: &ContextQuery,
		actor_id: ActorId,
	) -> Result<openresponses::request::Input> {
		let actor = query.actor(actor_id)?;

		let mut timestamped_items = actor
			.items()
			.iter()
			.xtry_map(|item_id| self.item_to_input(query, &actor, *item_id))?;

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
		query: &ContextQuery,
		actor: &Actor,
		item_id: ItemId,
	) -> Result<(Timestamp, Vec<openresponses::request::InputItem>)> {
		let item = query.item(item_id)?;
		let role = item_message_role(actor, item);

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
}

/// Get the message role for this actor, relative to the items actor id.
/// This is useful when an agent is constructing its context for an
/// openresponses request.
fn item_message_role(actor: &Actor, item: &Item) -> openresponses::MessageRole {
	use openresponses::MessageRole;
	match actor.kind() {
		ActorKind::System => MessageRole::System,
		ActorKind::Developer => MessageRole::Developer,
		ActorKind::Human => MessageRole::User,
		ActorKind::Agent => {
			if actor.id() == item.owner() {
				MessageRole::Assistant
			} else {
				MessageRole::User
			}
		}
	}
}
