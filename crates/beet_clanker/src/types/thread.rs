use crate::prelude::*;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;

pub type ThreadId = Uuid7<Thread>;

#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Serialize,
	Deserialize,
)]
pub struct Thread {
	id: ThreadId,
	/// A list of all actors, to which this thread is subscribed
	actors: Vec<ActorId>,
	// /// A list of all threads, to which this thread is subscribed.
	// threads: Vec<ThreadId>,
	/// A list of items that may have been created by this
	/// actor or others, to be included in clanker context
	/// or rendered in ui.
	///
	/// ## Sorting
	/// This list is sorted by [`ItemId`] which, as a `uuid::v7`,
	/// is basically chronological within a millisecond, and
	/// not accounting for clock skew in the case of distributed
	/// creation. This is generally acceptible for the use-case of
	/// agent context.
	items: Vec<ItemId>,
	deny_items: Vec<ItemKind>,
}

impl Document for Thread {
	type Id = ThreadId;
	fn id(&self) -> Self::Id { self.id }
}


impl Thread {
	/// Deny all non-display items like reasoning and function calls
	pub fn display_only() -> Self {
		Self {
			deny_items: ItemKind::non_display_kinds(),
			..default()
		}
	}


	pub fn items(&self) -> &[ItemId] { &self.items }
	pub fn actors(&self) -> &[ActorId] { &self.actors }

	pub fn with_actors(
		mut self,
		actor_ids: impl IntoIterator<Item = ActorId>,
	) -> Self {
		for actor_id in actor_ids {
			if !self.actors.contains(&actor_id) {
				self.actors.push(actor_id);
			}
		}
		self
	}

	pub fn with_actor(mut self, actor_id: ActorId) -> Self {
		if !self.actors.contains(&actor_id) {
			self.actors.push(actor_id);
		}
		self
	}
	pub fn insert_actor(&mut self, actor_id: ActorId) {
		if !self.actors.contains(&actor_id) {
			self.actors.push(actor_id);
		}
	}

	/// Get the list of items created after the given item, sorted by creation time.
	pub fn items_after(&self, item_id: ItemId) -> &[ItemId] {
		match self.items.binary_search(&item_id) {
			Ok(i) => &self.items[i + 1..],
			Err(i) => &self.items[i..],
		}
	}
	/// Add the item to the list, maintaining uniqueness and sort order.
	/// Returns `true` if the item was inserted.
	pub fn try_push(&mut self, item: &Item) -> bool {
		if self.deny_items.contains(&item.content().kind())
			|| !self.actors.contains(&item.owner())
		{
			return false;
		}

		let item_id = item.id();

		if let Some(last) = self.items.last() {
			// usually already sorted
			if item_id >= *last {
				self.items.push(item_id);
				return true;
			}
		}
		match self.items.binary_search(&item_id) {
			Ok(_) => {
				// already in the list, do nothing
				false
			}
			Err(i) => {
				self.items.insert(i, item_id);
				true
			}
		}
	}
}
