use crate::prelude::*;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;


#[derive(
	Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, Resource,
)]
pub struct ContextMap {
	actors: HashMap<ActorId, Actor>,
	/// List of items loaded in memory
	items: HashMap<ItemId, Item>,
}


impl ContextMap {
	pub fn add_actor(&mut self, actor: Actor) {
		self.actors.insert(actor.id(), actor);
	}
	pub fn remove_actor(&mut self, actor_id: ActorId) -> Option<Actor> {
		self.actors.remove(&actor_id)
	}
	pub fn add_item(&mut self, item: Item) {
		self.items.insert(item.id(), item);
	}
	pub fn remove_item(&mut self, item_id: ItemId) -> Option<Item> {
		self.items.remove(&item_id)
	}

	pub fn actor(&self, actor_id: ActorId) -> Result<&Actor> {
		self.actors.get(&actor_id).ok_or_else(|| {
			bevyhow!("ActorId {actor_id} not found in ContextMap")
		})
	}
	pub fn actor_mut(&mut self, actor_id: ActorId) -> Result<&mut Actor> {
		self.actors.get_mut(&actor_id).ok_or_else(|| {
			bevyhow!("ActorID {actor_id} not found in ContextMap")
		})
	}
	pub fn item(&self, item_id: ItemId) -> Result<&Item> {
		self.items
			.get(&item_id)
			.ok_or_else(|| bevyhow!("ItemId {item_id} not found in ContextMap"))
	}
	pub fn item_mut(&mut self, item_id: ItemId) -> Result<&mut Item> {
		self.items
			.get_mut(&item_id)
			.ok_or_else(|| bevyhow!("ItemId {item_id} not found in ContextMap"))
	}
}


#[derive(SystemParam)]
pub struct ContextQuery<'w, 's> {
	pub commands: Commands<'w, 's>,
	pub context_map: ResMut<'w, ContextMap>,
	pub ancestors: Query<'w, 's, &'static ChildOf>,
	pub children: Query<'w, 's, &'static Children>,
	pub actors: Query<'w, 's, (Entity, &'static ActorId)>,
}
impl ContextQuery<'_, '_> {
	pub fn add_actor(&mut self, actor: Actor) -> ActorId {
		let id = actor.id();
		self.context_map.add_actor(actor);
		id
	}

	pub fn actor(&self, actor_id: ActorId) -> Result<&Actor> {
		self.context_map.actor(actor_id)
	}
	pub fn actor_mut(&mut self, actor_id: ActorId) -> Result<&mut Actor> {
		self.context_map.actor_mut(actor_id)
	}
	pub fn actors(&self) -> impl Iterator<Item = &Actor> {
		self.context_map.actors.values()
	}

	pub fn actor_entities(&self, actor_id: ActorId) -> Vec<Entity> {
		self.actors
			.iter()
			.filter_map(|(entity, other_id)| match &actor_id == other_id {
				true => Some(entity),
				false => None,
			})
			.collect()
	}


	pub fn item(&self, item_id: ItemId) -> Result<&Item> {
		self.context_map.item(item_id)
	}
	pub fn item_mut(&mut self, item_id: ItemId) -> Result<&mut Item> {
		self.context_map.item_mut(item_id)
	}

	pub fn add_item(&mut self, item: Item) -> Result<&mut Self> {
		let item_id = item.id();
		let item_scope = item.scope();
		let actor_id = item.owner();
		match item_scope {
			ItemScope::Actor => {
				self.actor_mut(actor_id)?.push(item_id);
			}
			ItemScope::ActorList(actor_list) => {
				for actor_id in actor_list {
					self.actor_mut(*actor_id)?.push(item_id);
				}
			}
			ItemScope::Family => {
				let actor_entities = self.actor_entities(actor_id);
				for actor_entity in actor_entities {
					let root = self.ancestors.root_ancestor(actor_entity);
					for (_, actor_id) in self
						.children
						.iter_descendants_inclusive(root)
						.filter_map(|entity| self.actors.get(entity).ok())
					{
						self.context_map.actor_mut(*actor_id)?.push(item_id);
					}
				}
			}
			ItemScope::World => {
				for (_, actor_id) in self.actors.iter() {
					self.context_map.actor_mut(*actor_id)?.push(item_id);
				}
			}
		}
		self.context_map.add_item(item);
		Ok(self)
	}
}
