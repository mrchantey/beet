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

	pub fn actors(&self) -> impl Iterator<Item = &Actor> {
		self.actors.values()
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
	actors: Query<'w, 's, (Entity, &'static ActorId)>,
}

impl std::ops::Deref for ContextQuery<'_, '_> {
	type Target = ContextMap;
	fn deref(&self) -> &Self::Target { &self.context_map }
}
impl std::ops::DerefMut for ContextQuery<'_, '_> {
	fn deref_mut(&mut self) -> &mut Self::Target { self.context_map.as_mut() }
}

impl ContextQuery<'_, '_> {
	pub fn add_actor(&mut self, actor: Actor) -> ActorId {
		let id = actor.id();
		self.context_map.add_actor(actor);
		id
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

	pub fn actor_items(
		&self,
		actor_id: ActorId,
		items_after: Option<ItemId>,
	) -> Result<Vec<&Item>> {
		let actor = self.actor(actor_id)?;
		match items_after {
			Some(item_id) => actor.items_after(item_id),
			None => actor.items(),
		}
		.into_iter()
		.map(|item_id| self.item(*item_id))
		.collect()
	}

	/// Adds items to actors based on their scope,
	/// and triggers an `ItemsAdded` event for each affected actor entity.
	pub fn add_items(
		&mut self,
		items: impl IntoIterator<Item = Item>,
	) -> Result<()> {
		let mut actor_map = MultiMap::<ActorId, ItemId>::new();


		let mut all_ids = Vec::new();
		for item in items {
			let item_id = item.id();
			all_ids.push(item_id);
			for actor in self.add_item(item)? {
				actor_map.insert(actor, item_id);
			}
		}

		for (actor_id, items) in actor_map.into_iter_all() {
			for entity in self.actor_entities(actor_id) {
				self.commands.trigger(ActorItemsAdded {
					entity,
					actor_id,
					items: items.clone(),
				});
			}
		}

		self.commands.trigger(ItemsAdded { items: all_ids });

		Ok(())
	}

	/// Adds an item to the actors based on the provided scope,
	/// and returns the actors who had the item added.
	/// This excludes actors who already own the item.
	fn add_item(&mut self, item: Item) -> Result<Vec<ActorId>> {
		let item_id = item.id();
		let item_scope = item.scope();
		let owner_id = item.owner();
		let actors_to_add = match item_scope {
			ItemScope::Owner => {
				vec![owner_id]
			}
			ItemScope::ActorList(actor_list) => actor_list.clone(),
			ItemScope::Family => {
				let actor_entities = self.actor_entities(owner_id);
				let mut actor_ids = HashSet::new();
				for actor_entity in actor_entities {
					let root = self.ancestors.root_ancestor(actor_entity);
					for (_, actor_id) in self
						.children
						.iter_descendants_inclusive(root)
						.filter_map(|entity| self.actors.get(entity).ok())
					{
						actor_ids.insert(*actor_id);
					}
				}
				actor_ids.into_iter().collect()
			}
			ItemScope::World => {
				self.actors.iter().map(|(_, actor_id)| *actor_id).collect()
			}
		};
		self.context_map.add_item(item);

		let mut added = Vec::new();
		for actor in actors_to_add {
			if let Ok(actor) = self.actor_mut(actor) {
				if actor.push(item_id) {
					added.push(actor.id());
				}
			}
		}

		Ok(added)
	}
}

/// Called on each actor entity with a list of items added
#[derive(EntityEvent)]
pub struct ActorItemsAdded {
	pub actor_id: ActorId,
	pub entity: Entity,
	pub items: Vec<ItemId>,
}

#[derive(Event)]
pub struct ItemsAdded {
	pub items: Vec<ItemId>,
}
