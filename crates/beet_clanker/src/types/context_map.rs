use crate::openresponses::request::Input;
use crate::openresponses::request::InputItem;
use crate::openresponses::request::MessageContent;
use crate::openresponses::request::MessageParam;
use crate::prelude::*;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::sync::atomic::AtomicU64;



#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Resource)]
pub struct ContextMap {
	actors: HashMap<ActorId, Entity>,
	items: HashMap<ItemId, Entity>,
}


impl ContextMap {
	pub(super) fn add_actor(&mut self, actor_id: ActorId, entity: Entity) {
		self.actors.insert(actor_id, entity);
	}
	pub(super) fn add_item(&mut self, item_id: ItemId, entity: Entity) {
		self.items.insert(item_id, entity);
	}

	pub fn actor(&self, actor_id: ActorId) -> Result<Entity> {
		self.actors.get(&actor_id).copied().ok_or_else(|| {
			bevyhow!("ActorId {actor_id} not found in ContextMap")
		})
	}
	pub fn item(&self, item_id: ItemId) -> Result<Entity> {
		self.items
			.get(&item_id)
			.copied()
			.ok_or_else(|| bevyhow!("ItemId {item_id} not found in ContextMap"))
	}
}


#[derive(SystemParam)]
pub struct ContextQuery<'w, 's> {
	context_map: Res<'w, ContextMap>,
	actors: Query<'w, 's, &'static Actor>,
	items: Query<'w, 's, &'static Item>,
}
impl ContextQuery<'_, '_> {
	pub fn actor(&self, actor_id: ActorId) -> Result<&Actor> {
		self.context_map
			.actor(actor_id)
			.and_then(|entity| self.actors.get(entity)?.xok())
	}
	pub fn item(&self, item_id: ItemId) -> Result<&Item> {
		self.context_map
			.item(item_id)
			.and_then(|entity| self.items.get(entity)?.xok())
	}

	pub fn build_input(
		&self,
		actor_id: ActorId,
	) -> Result<openresponses::request::Input> {
		let actor = self.actor(actor_id)?;

		let items = actor
			.items()
			.iter()
			.xtry_map(|item_id| self.item_to_input(&actor, *item_id))?
			.into_iter()
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
		actor: &Actor,
		item_id: ItemId,
	) -> Result<Vec<openresponses::request::InputItem>> {
		let item = self.item(item_id)?;
		let role = actor.relative_message_role(item.actor_id());


		match item.content() {
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
			Content::File(file_content) => todo!(),
			Content::FunctionCall(function_call) => {
				static CALL_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

				let call_id = CALL_ID_COUNTER
					.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

				vec![
					InputItem::FunctionCall(openresponses::FunctionCallParam {
						id: None,
						role,
						function_name: function_call
							.function_name()
							.to_string(),
						args: function_call.args().clone(),
						call_id: Some(call_id),
					}),
					InputItem::FunctionCallOutput(
						openresponses::request::FunctionCallOutputParam {
							id: None,
							role,
							output: function_call.output().clone(),
							call_id: Some(call_id),
						},
					),
				]
			}
		}
		.xok()
	}
}
