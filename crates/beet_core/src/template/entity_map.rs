//! [`TemplateEntityMap`]: the file-key to world-entity map a document retains.
use crate::prelude::*;

/// The map between a document's **file keys** and the world entities they built
/// into, retained for the document's lifetime on the loaded root, or on the host
/// of a [`SceneDocument`](super::SceneDocument).
///
/// A serialized entity is keyed by an [`EntityKey`], so a first save reads
/// `"0"`, `"1"`, `"2"`. Stability across an edit comes from this map, not from
/// the encoding: it is built during the load and every save goes back through
/// it, so an entity that was loaded writes its **original** key and only a
/// genuinely new entity mints a fresh one. A save therefore never derives keys
/// from live entity bits, which is what stops a reopen from silently rewriting
/// every reference in the file.
///
/// It is a cache, so it never serializes: it carries no reflect registration,
/// and the extractor only ever dumps registered types. It is rebuilt from
/// scratch by each load.
#[derive(Debug, Default, Clone, Component)]
pub struct TemplateEntityMap {
	/// File key to the world entity it built into.
	to_world: HashMap<EntityKey, Entity>,
	/// World entity back to the file key it was loaded from.
	to_file: HashMap<Entity, EntityKey>,
	/// The lowest number never yet minted or read as a key, so a new entity
	/// cannot collide with a key some other entity still holds.
	next_index: u32,
}

impl TemplateEntityMap {
	/// Build a map from the `(file key, world entity)` pairs of one load.
	pub fn from_pairs(
		pairs: impl IntoIterator<Item = (EntityKey, Entity)>,
	) -> Self {
		let mut map = Self::default();
		for (key, entity) in pairs {
			map.insert(key, entity);
		}
		map
	}

	/// Record that `file_key` built into `entity`.
	pub fn insert(&mut self, file_key: impl Into<EntityKey>, entity: Entity) {
		let file_key = file_key.into();
		if let Some(index) = file_key.index() {
			self.next_index = self.next_index.max(index.saturating_add(1));
		}
		self.to_world.insert(file_key.clone(), entity);
		self.to_file.insert(entity, file_key);
	}

	/// Forget `file_key` and the entity it built into, ie an entity the document
	/// removed. The key is never reused: [`file_key`](Self::file_key) keeps
	/// minting above it, so a later reference to it can only be a dangling one.
	pub fn remove(&mut self, file_key: &EntityKey) -> Option<Entity> {
		let entity = self.to_world.remove(file_key)?;
		self.to_file.remove(&entity);
		Some(entity)
	}

	/// The world entity a file key built into, if this document loaded one.
	pub fn world(&self, file_key: impl Into<EntityKey>) -> Option<Entity> {
		self.to_world.get(&file_key.into()).copied()
	}

	/// The file key `entity` was loaded under, if it was.
	pub fn get_file_key(&self, entity: Entity) -> Option<&EntityKey> {
		self.to_file.get(&entity)
	}

	/// The file key to save `entity` under: the one it loaded from, else a
	/// freshly minted key that no entity in this document has ever held.
	pub fn file_key(&mut self, entity: Entity) -> EntityKey {
		if let Some(key) = self.get_file_key(entity) {
			return key.clone();
		}
		let key = EntityKey::minted(self.next_index);
		self.insert(key.clone(), entity);
		key
	}

	/// The number of mapped entities.
	pub fn len(&self) -> usize { self.to_file.len() }

	/// Whether no entity is mapped.
	pub fn is_empty(&self) -> bool { self.to_file.is_empty() }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn loaded_keys_are_kept_and_new_ones_minted() {
		let mut world = World::new();
		let (first, second) =
			(world.spawn_empty().id(), world.spawn_empty().id());
		let mut map = TemplateEntityMap::from_pairs([
			(EntityKey::from(0), first),
			(EntityKey::from(1), second),
		]);

		// a loaded entity saves back under the key it came from, whatever its
		// live entity bits are
		map.file_key(second).xpect_eq(EntityKey::from(1));
		map.file_key(first).xpect_eq(EntityKey::from(0));
		map.world(1).unwrap().xpect_eq(second);

		// an entity this document has never seen mints the next unused key, and
		// keeps it on every later save
		let fresh = world.spawn_empty().id();
		map.file_key(fresh).xpect_eq(EntityKey::from(2));
		map.file_key(fresh).xpect_eq(EntityKey::from(2));
	}

	/// A sparse file (keys `0` and `7`) mints above every number in use, so a
	/// new entity can never collide with one a live entity still holds, and a
	/// key that is no number reserves none.
	#[crate::test]
	fn minting_never_collides() {
		let mut world = World::new();
		let mut map = TemplateEntityMap::from_pairs([
			(EntityKey::from(0), world.spawn_empty().id()),
			(EntityKey::from(7), world.spawn_empty().id()),
			(
				EntityKey::parse("main/p.9").unwrap(),
				world.spawn_empty().id(),
			),
		]);
		map.file_key(world.spawn_empty().id())
			.xpect_eq(EntityKey::from(8));
	}
}
