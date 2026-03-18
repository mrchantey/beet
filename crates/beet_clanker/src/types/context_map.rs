use crate::prelude::*;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;


#[derive(
	Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, Resource,
)]
pub struct ContextMap {
	actors: DocMap<Actor>,
	items: DocMap<Item>,
	threads: DocMap<Thread>,
}


impl ContextMap {
	pub fn actors(&self) -> &DocMap<Actor> { &self.actors }
	pub fn actors_mut(&mut self) -> &mut DocMap<Actor> { &mut self.actors }

	pub fn items(&self) -> &DocMap<Item> { &self.items }
	pub fn items_mut(&mut self) -> &mut DocMap<Item> { &mut self.items }

	pub fn threads(&self) -> &DocMap<Thread> { &self.threads }
	pub fn threads_mut(&mut self) -> &mut DocMap<Thread> { &mut self.threads }

	pub fn thread_items(
		&self,
		thread_id: ThreadId,
		items_after: Option<ItemId>,
	) -> Result<Vec<&Item>> {
		let thread = self.threads.get(thread_id)?;
		match items_after {
			Some(item_id) => thread.items_after(item_id),
			None => thread.items(),
		}
		.into_iter()
		.map(|item_id| self.items.get(*item_id))
		.collect()
	}
}


#[derive(SystemParam)]
pub struct ContextQuery<'w, 's> {
	pub commands: Commands<'w, 's>,
	pub context_map: ResMut<'w, ContextMap>,
	pub ancestors: Query<'w, 's, &'static ChildOf>,
	pub children: Query<'w, 's, &'static Children>,
	pub actor_query: Query<'w, 's, (Entity, &'static ActorId)>,
	pub thread_query: Query<'w, 's, (Entity, &'static ThreadId)>,
}

impl std::ops::Deref for ContextQuery<'_, '_> {
	type Target = ContextMap;
	fn deref(&self) -> &Self::Target { &self.context_map }
}
impl std::ops::DerefMut for ContextQuery<'_, '_> {
	fn deref_mut(&mut self) -> &mut Self::Target { self.context_map.as_mut() }
}

impl ContextQuery<'_, '_> {
	pub fn actor_entities(&self, actor_id: ActorId) -> Vec<Entity> {
		self.actor_query
			.iter()
			.filter_map(|(entity, other_id)| match &actor_id == other_id {
				true => Some(entity),
				false => None,
			})
			.collect()
	}

	pub fn response_complete(
		&mut self,
		response_id: impl Into<String>,
		interrupted: bool,
	) {
		self.commands.trigger(ResponseComplete {
			id: response_id.into(),
			interrupted,
		});
	}

	pub fn add_items<M>(
		&mut self,
		items: impl XIntoIterator<M, Item>,
	) -> Result<()> {
		for item in items.xinto_iter() {
			self.add_item(item)?;
		}
		Ok(())
	}

	fn add_item(&mut self, item: Item) -> Result {
		let item_id = item.id();

		// 1. try push to to threads
		let threads_changed = self
			.threads
			.values_mut()
			.xtry_filter(|thread| -> Result<bool> {
				thread.try_push(&item).xok()
			})?
			.into_iter()
			.map(|thread| thread.id())
			.collect::<Vec<_>>();

		// 2. insert item
		let exists = self.items.contains_key(item_id);
		self.items.insert(item);


		// 3. trigger events
		let changed_entities = self
			.thread_query
			.iter()
			.filter(|(_, thread_id)| threads_changed.contains(thread_id))
			.map(|(entity, _)| entity)
			.collect::<Vec<_>>();

		if !exists {
			self.commands.trigger(ItemCreated { item: item_id });
			for entity in changed_entities.iter() {
				self.commands.trigger(EntityItemCreated {
					entity: *entity,
					item: item_id,
				});
			}
		}

		self.commands.trigger(ItemUpdated { item: item_id });
		for entity in changed_entities.iter() {
			self.commands.trigger(EntityItemUpdated {
				entity: *entity,
				item: item_id,
			});
		}

		Ok(())
	}
}

/// Item created event, runs before [`EntityItemCreated`] and [`ItemUpdated`]
#[derive(Event)]
pub struct ItemCreated {
	pub item: ItemId,
}

/// Item created event, runs before [`EntityItemUpdated`]
#[derive(EntityEvent)]
pub struct EntityItemCreated {
	pub entity: Entity,
	pub item: ItemId,
}

#[derive(Event)]
pub struct ItemUpdated {
	pub item: ItemId,
}

#[derive(EntityEvent)]
pub struct EntityItemUpdated {
	pub entity: Entity,
	pub item: ItemId,
}

#[derive(Event)]
pub struct ResponseComplete {
	/// The openresponses id for this response
	id: String,
	interrupted: bool,
}
