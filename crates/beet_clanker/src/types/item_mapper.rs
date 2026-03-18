use crate::openresponses::ContentPart;
use crate::openresponses::FunctionCallStatus;
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
pub struct ItemMapper {
	/// Map an openresponses call id to an [`ItemId`]
	call_id_to_item_id: HashMap<String, ItemId>,
	item_id_to_call_id: HashMap<ItemId, String>,
	/// map responses output items to an [`ItemId`]
	responses_item_map: HashMap<PartialItemKey, ItemId>,
}


impl ItemMapper {
	pub fn new() -> Self {
		Self {
			call_id_to_item_id: default(),
			item_id_to_call_id: default(),
			responses_item_map: default(),
		}
	}
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

	fn set_call_id(&mut self, item_id: ItemId, call_id: String) -> Result {
		if self.item_id_to_call_id.contains_key(&item_id) {
			bevybail!("item_id {item_id} already has a call_id");
		} else if self.call_id_to_item_id.contains_key(&call_id) {
			bevybail!("call_id {call_id} already has an item_id");
		}
		self.call_id_to_item_id.insert(call_id.clone(), item_id);
		self.item_id_to_call_id.insert(item_id, call_id);
		Ok(())
	}
	fn get_call_id(&self, item_id: ItemId) -> Result<String> {
		self.item_id_to_call_id
			.get(&item_id)
			.cloned()
			.ok_or_else(|| {
				bevyhow!("no call_id registered for item_id {item_id}")
			})
	}
	fn get_item_id(&self, call_id: &str) -> Result<ItemId> {
		self.call_id_to_item_id
			.get(call_id)
			.cloned()
			.ok_or_else(|| {
				bevyhow!("no item_id registered for call_id {call_id}")
			})
	}

	fn set_response_item(
		&mut self,
		key: PartialItemKey,
		item_id: ItemId,
	) -> Result {
		if self.responses_item_map.contains_key(&key) {
			bevybail!("responses item map already has content for key {key:?}");
		} else {
			self.responses_item_map.insert(key, item_id);
		}
		Ok(())
	}

	pub(super) fn get_response_item(
		&self,
		key: &PartialItemKey,
	) -> Result<ItemId> {
		self.responses_item_map.get(key).cloned().ok_or_else(|| {
			bevyhow!("no item_id registered for responses item key {key:?}")
		})
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
					call_id: self.get_call_id(item.id())?,
					name: function_call.function_name().to_string(),
					arguments: function_call.args().to_string(),
					status: None,
				})
			}
			Content::FunctionCallOutput(output_item) => {
				InputItem::FunctionCallOutput(FunctionCallOutputParam {
					id: None,
					call_id: self
						.get_call_id(output_item.function_call_item)?,
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
		&mut self,
		owner: ActorId,
		items: impl IntoIterator<Item = OutputItem>,
	) -> Result<Vec<Item>> {
		items
			.into_iter()
			.xtry_map(|item| self.from_openresponses_output(owner, item))?
			.into_iter()
			.flatten()
			.collect::<Vec<_>>()
			.xok()
	}

	/// Self must be mutable to update id maps
	fn from_openresponses_output(
		&mut self,
		owner: ActorId,
		responses_item: OutputItem,
	) -> Result<Vec<Item>> {
		let mut out = Vec::new();
		match responses_item {
			OutputItem::Message(message) => {
				let status = match message.status {
					MessageStatus::InProgress => ItemStatus::InProgress,
					MessageStatus::Completed => ItemStatus::Completed,
					MessageStatus::Incomplete => ItemStatus::Interrupted,
				};
				for (content_index, content) in
					message.content.into_iter().enumerate()
				{
					let item = match content {
						OutputContent::OutputText(output_text) => {
							if !output_text.annotations.is_empty() {
								todo!("inline annotations as markdown links");
							}
							Item::new(owner, status, TextItem(output_text.text))
						}
						OutputContent::Refusal(refusal) => Item::new(
							owner,
							status,
							RefusalItem(refusal.refusal),
						),
					};
					self.set_response_item(
						PartialItemKey::Content {
							responses_id: message.id.clone(),
							content_index: content_index as u32,
						},
						item.id(),
					)?;
					out.push(item);
				}
			}
			OutputItem::FunctionCall(fc_call) => {
				let status = function_call_status(fc_call.status);
				let item = Item::new(owner, status, FunctionCallItem {
					name: fc_call.name,
					arguments: fc_call.arguments,
				});
				self.set_call_id(item.id(), fc_call.call_id)?;
				self.set_response_item(
					PartialItemKey::Single {
						responses_id: fc_call.id,
					},
					item.id(),
				)?;
				out.push(item);
			}
			OutputItem::FunctionCallOutput(fc_output) => {
				let status = function_call_status(Some(fc_output.status));
				let item = Item::new(owner, status, FunctionCallOutputItem {
					function_call_item: self.get_item_id(&fc_output.call_id)?,
					output: fc_output.output,
				});
				self.set_response_item(
					PartialItemKey::Single {
						responses_id: fc_output.id,
					},
					item.id(),
				)?;
				out.push(item);
			}
			OutputItem::Reasoning(reasoning_item) => {
				// note: we disregard encrypted content
				for (index, content) in
					reasoning_item.content.into_iter().enumerate()
				{
					let item = Item::new(
						owner,
						ItemStatus::Completed,
						ReasoningContentItem(content.text),
					);
					self.set_response_item(
						PartialItemKey::Content {
							responses_id: reasoning_item.id.clone(),
							content_index: index as u32,
						},
						item.id(),
					)?;
					out.push(item);
				}
				for (index, summary) in
					reasoning_item.summary.into_iter().enumerate()
				{
					let item = Item::new(
						owner,
						ItemStatus::Completed,
						ReasoningSummaryItem(summary.text),
					);
					self.set_response_item(
						PartialItemKey::ReasoningSummary {
							responses_id: reasoning_item.id.clone(),
							summary_index: index as u32,
						},
						item.id(),
					)?;
					out.push(item);
				}
			}
		}
		out.xok()
	}
}

fn function_call_status(status: Option<FunctionCallStatus>) -> ItemStatus {
	match status {
		Some(FunctionCallStatus::InProgress) => ItemStatus::InProgress,
		Some(FunctionCallStatus::Completed) => ItemStatus::Completed,
		Some(FunctionCallStatus::Incomplete) => ItemStatus::Interrupted,
		// completed? i have no idea
		None => ItemStatus::Completed,
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
