use crate::prelude::*;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;


#[derive(
	Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, Resource,
)]
pub struct ContextMap {
	actors: HashMap<ActorId, Entity>,
	/// List of items loaded in memory
	items: HashMap<ItemId, Item>,
}


impl ContextMap {
	pub(super) fn add_actor(&mut self, actor_id: ActorId, entity: Entity) {
		self.actors.insert(actor_id, entity);
	}
	pub(super) fn remove_actor(&mut self, actor_id: ActorId) -> Option<Entity> {
		self.actors.remove(&actor_id)
	}
	// fn add_item(&mut self, item: Item) { self.items.insert(item.id(), item); }
	// fn remove_item(&mut self, item_id: ItemId) -> Option<Item> {
	// 	self.items.remove(&item_id)
	// }

	pub fn actor(&self, actor_id: ActorId) -> Result<Entity> {
		self.actors.get(&actor_id).copied().ok_or_else(|| {
			bevyhow!("ActorId {actor_id} not found in ContextMap")
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
	commands: Commands<'w, 's>,
	context_map: ResMut<'w, ContextMap>,
	ancestors: Query<'w, 's, &'static ChildOf>,
	children: Query<'w, 's, &'static Children>,
	actors: Query<'w, 's, (Entity, &'static mut Actor)>,
}
impl ContextQuery<'_, '_> {
	pub fn spawn_actor(&mut self, actor: Actor) -> EntityCommands<'_> {
		let id = actor.id();
		let entity = self.commands.spawn(actor);
		self.context_map.add_actor(id, entity.id());
		println!("Spawned actor {id} at entity {:?}", entity.id());
		entity
	}

	pub fn actor(&self, actor_id: ActorId) -> Result<&Actor> {
		self.context_map
			.actor(actor_id)
			.and_then(|entity| self.actors.get(entity)?.1.xok())
	}
	pub fn actor_mut(&mut self, actor_id: ActorId) -> Result<Mut<'_, Actor>> {
		self.context_map
			.actor(actor_id)
			.and_then(|entity| self.actors.get_mut(entity)?.1.xok())
	}
	pub fn item(&self, item_id: ItemId) -> Result<&Item> {
		self.context_map.item(item_id)
	}
	pub fn item_mut(&mut self, item_id: ItemId) -> Result<&mut Item> {
		self.context_map.item_mut(item_id)
	}

	pub fn add_item(&mut self, item: Item) -> Result {
		let item_id = item.id();
		let item_scope = item.scope();
		let owner = self.context_map.actor(item.owner())?;
		match item_scope {
			ItemScope::Actor => {
				self.actors.get_mut(owner)?.1.push(item_id);
			}
			ItemScope::ActorList(actor_list) => {
				for actor_id in actor_list {
					let actor_entity = self.context_map.actor(*actor_id)?;
					self.actors.get_mut(actor_entity)?.1.push(item_id);
				}
			}
			ItemScope::Family => {
				let root = self.ancestors.root_ancestor(owner);
				for entity in self.children.iter_descendants_inclusive(root) {
					let Ok((_, mut actor)) = self.actors.get_mut(entity) else {
						continue;
					};
					actor.push(item_id);
				}
			}
			ItemScope::World => {
				for (_, mut actor) in self.actors.iter_mut() {
					actor.push(item_id);
				}
			}
		}
		Ok(())
	}
}
