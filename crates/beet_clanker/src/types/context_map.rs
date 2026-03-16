use crate::prelude::*;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;


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
	commands: Commands<'w, 's>,
	context_map: Res<'w, ContextMap>,
	actors: Query<'w, 's, (Entity, &'static mut Actor)>,
	items: Query<'w, 's, (Entity, &'static mut Item)>,
}
impl ContextQuery<'_, '_> {
	pub fn reborrow(&mut self) -> ContextQuery<'_, '_> {
		ContextQuery {
			commands: self.commands.reborrow(),
			context_map: Res::clone(&self.context_map),
			actors: self.actors.reborrow(),
			items: self.items.reborrow(),
		}
	}

	pub fn actor(&self, actor_id: ActorId) -> Result<&Actor> {
		self.context_map
			.actor(actor_id)
			.and_then(|entity| self.actors.get(entity)?.1.xok())
	}
	pub fn item(&self, item_id: ItemId) -> Result<&Item> {
		self.context_map
			.item(item_id)
			.and_then(|entity| self.items.get(entity)?.1.xok())
	}
	pub fn actor_mut(&mut self, actor_id: ActorId) -> Result<Mut<'_, Actor>> {
		self.context_map
			.actor(actor_id)
			.and_then(|entity| self.actors.get_mut(entity)?.1.xok())
	}
	pub fn item_mut(&mut self, item_id: ItemId) -> Result<Mut<'_, Item>> {
		self.context_map
			.item(item_id)
			.and_then(|entity| self.items.get_mut(entity)?.1.xok())
	}



	/// Items that do not appear in any [`Actor::context`] will never be
	/// used, so should be despawned.
	pub fn despawn_orphan_items(&mut self) -> Result<&mut Self> {
		let parented_items = self
			.actors
			.iter()
			.flat_map(|(_, actor)| actor.unsorted_context())
			.collect::<HashSet<_>>();

		let orphan_items = self
			.items
			.iter()
			.filter(|(_, item)| !parented_items.contains(&item.id()))
			.map(|(e, _)| e)
			.collect::<Vec<_>>();

		for entity in orphan_items {
			self.commands.entity(entity).despawn();
		}
		drop(parented_items);
		self.xok()
	}
}
