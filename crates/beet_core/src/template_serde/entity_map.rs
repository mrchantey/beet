//! [`TemplateEntityMap`]: the file-key to world-entity map a document retains.
use crate::prelude::*;
use bevy::ecs::entity::EntityMapper;

/// The map between a document's **file keys** and the world entities they built
/// into, retained for the document's lifetime on the loaded root, or on the host
/// of a [`SceneDocument`](super::SceneDocument).
///
/// A serialized entity is keyed by its in-file [`Entity`], generation stripped, so
/// a file reads `0`, `1`, `2`. Stability across an edit comes from this map, not
/// from the encoding: it is built during the load and every save goes back
/// through it, so an entity that was loaded writes its **original** key and only a
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
	to_world: HashMap<u32, Entity>,
	/// World entity back to the file key it was loaded from.
	to_file: HashMap<Entity, u32>,
	/// The lowest key never yet handed out, so a new entity cannot collide with a
	/// key some other entity still holds.
	next_key: u32,
}

impl TemplateEntityMap {
	/// Build a map from the `(file key, world entity)` pairs of one load.
	pub fn from_pairs(pairs: impl IntoIterator<Item = (u32, Entity)>) -> Self {
		let mut map = Self::default();
		for (key, entity) in pairs {
			map.insert(key, entity);
		}
		map
	}

	/// Record that `file_key` built into `entity`.
	pub fn insert(&mut self, file_key: u32, entity: Entity) {
		self.to_world.insert(file_key, entity);
		self.to_file.insert(entity, file_key);
		self.next_key = self.next_key.max(file_key + 1);
	}

	/// Forget `file_key` and the entity it built into, ie an entity the document
	/// removed. The key is never reused: [`file_key`](Self::file_key) keeps
	/// minting above it, so a later reference to it can only be a dangling one.
	pub fn remove(&mut self, file_key: u32) -> Option<Entity> {
		let entity = self.to_world.remove(&file_key)?;
		self.to_file.remove(&entity);
		Some(entity)
	}

	/// The world entity a file key built into, if this document loaded one.
	pub fn world(&self, file_key: u32) -> Option<Entity> {
		self.to_world.get(&file_key).copied()
	}

	/// The file key `entity` was loaded under, if it was.
	pub fn get_file_key(&self, entity: Entity) -> Option<u32> {
		self.to_file.get(&entity).copied()
	}

	/// The file key to save `entity` under: the one it loaded from, else a
	/// freshly minted key that no entity in this document has ever held.
	pub fn file_key(&mut self, entity: Entity) -> u32 {
		if let Some(key) = self.get_file_key(entity) {
			return key;
		}
		let key = self.next_key;
		self.insert(key, entity);
		key
	}

	/// The number of mapped entities.
	pub fn len(&self) -> usize { self.to_file.len() }

	/// Whether no entity is mapped.
	pub fn is_empty(&self) -> bool { self.to_file.is_empty() }

	/// The file-side [`Entity`] to save `entity` under, ie its
	/// [`file_key`](Self::file_key) with no generation.
	pub fn file_entity(&mut self, entity: Entity) -> Entity {
		// a placeholder names no entity, so it must not mint a key for one.
		if entity == Entity::PLACEHOLDER {
			return Entity::PLACEHOLDER;
		}
		Entity::from_raw_u32(self.file_key(entity))
			.unwrap_or(Entity::PLACEHOLDER)
	}
}

/// Maps live world entities to their file entities, the save-side counterpart of
/// the build path's reference mapper.
///
/// An `Entity`-typed component field (a [`ChildOf`], a cross-entity reference) is
/// an entity reference, so it is written as the target's file key exactly like the
/// file keys are, rather than as whatever bits the target happens to hold this
/// run.
pub(super) struct FileEntityMapper<'a>(pub &'a mut TemplateEntityMap);

impl EntityMapper for FileEntityMapper<'_> {
	fn get_mapped(&mut self, source: Entity) -> Entity {
		self.0.file_entity(source)
	}
	fn set_mapped(&mut self, _source: Entity, _target: Entity) {}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn loaded_keys_are_kept_and_new_ones_minted() {
		let mut world = World::new();
		let (first, second) =
			(world.spawn_empty().id(), world.spawn_empty().id());
		let mut map = TemplateEntityMap::from_pairs([(0, first), (1, second)]);

		// a loaded entity saves back under the key it came from, whatever its
		// live entity bits are
		map.file_key(second).xpect_eq(1);
		map.file_key(first).xpect_eq(0);
		map.world(1).unwrap().xpect_eq(second);

		// an entity this document has never seen mints the next unused key, and
		// keeps it on every later save
		let fresh = world.spawn_empty().id();
		map.file_key(fresh).xpect_eq(2);
		map.file_key(fresh).xpect_eq(2);
	}

	/// A sparse file (keys `0` and `7`) mints above every key in use, so a new
	/// entity can never collide with one a live entity still holds.
	#[crate::test]
	fn minting_never_collides() {
		let mut world = World::new();
		let mut map = TemplateEntityMap::from_pairs([
			(0, world.spawn_empty().id()),
			(7, world.spawn_empty().id()),
		]);
		map.file_key(world.spawn_empty().id()).xpect_eq(8);
	}

	#[crate::test]
	fn placeholder_mints_nothing() {
		let mut map = TemplateEntityMap::default();
		map.file_entity(Entity::PLACEHOLDER)
			.xpect_eq(Entity::PLACEHOLDER);
		map.is_empty().xpect_true();
	}
}

/// Conformance tests for the serialized format's identity guarantees, run
/// against the real [`TemplateSaver`] and [`TemplateLoader`] rather than a
/// hand-written fixture, so they pin what those actually emit.
#[cfg(all(test, feature = "json"))]
mod conformance {
	use crate::prelude::*;

	/// A world that can round-trip a named hierarchy.
	fn world() -> World { <(TemplatePlugin, MinimalTypesPlugin)>::world() }

	/// A parent with two named children, the smallest tree with hierarchy,
	/// order and a cross-entity reference.
	fn spawn_tree(world: &mut World) -> Entity {
		world
			.spawn((Name::new("parent"), children![
				Name::new("a"),
				Name::new("b"),
			]))
			.flush()
	}

	fn save(world: &mut World, root: Entity) -> String {
		TemplateSaver::new()
			.save_roots(world, MediaType::Json, [root])
			.unwrap()
			.as_utf8()
			.unwrap()
			.to_string()
	}

	/// The file keys in **file order**, read back through the real
	/// deserializer.
	///
	/// A `serde_json::Value` would lose that order (its map sorts keys as
	/// strings, so `"10"` lands before `"2"`), and order is the children-order
	/// contract: the build path applies each `ChildOf` in file order, so file
	/// order *is* child order.
	fn entity_keys(world: &World, json: &str) -> Vec<u32> {
		use serde::de::DeserializeSeed;
		let registry = world.resource::<AppTypeRegistry>().read();
		DynamicTemplateDeserializer {
			type_registry: &registry,
		}
		.deserialize(&mut serde_json::Deserializer::from_str(json))
		.unwrap()
		.entities
		.iter()
		.map(|entity| entity.entity.index_u32())
		.collect()
	}

	/// The observed encoding, straight off the saver: file keys are the
	/// generation-stripped file keys `0`, `1`, `2`, and a `ChildOf` holds the
	/// *parent's file key*, not whatever bits it happens to hold this run.
	#[crate::test]
	fn format_probe() {
		let mut world = world();
		let root = spawn_tree(&mut world);
		let text = save(&mut world, root);
		let json: serde_json::Value = serde_json::from_str(&text).unwrap();

		let entities = json["entities"].as_object().unwrap();
		entities.len().xpect_eq(3);
		for key in ["0", "1", "2"] {
			entities.contains_key(key).xpect_true();
		}
		// the root carries no parent, each child points at file key 0
		entities["0"]["components"]
			.get("bevy_ecs::hierarchy::ChildOf")
			.xpect_none();
		// an `Entity`-typed field is serialized by bevy as `Entity::to_bits`,
		// which is opaque (a `NonMaxU32` index is stored inverted) and which
		// bevy explicitly does not guarantee across versions. Pinning the exact
		// integer here turns a bevy change to that encoding into a loud test
		// failure rather than a silent break of every saved document.
		let parent_ref = Entity::from_raw_u32(0).unwrap().to_bits();
		parent_ref.xpect_eq(4294967295);
		for child in ["1", "2"] {
			entities[child]["components"]["bevy_ecs::hierarchy::ChildOf"]
				.as_u64()
				.unwrap()
				.xpect_eq(parent_ref);
		}
	}

	/// Save, load, save: the second save is byte-identical to the first.
	///
	/// This is the whole point of the retained map. Without it the reload's
	/// fresh entity bits would leak into the file and every reopen would produce
	/// a different document that happens to mean the same thing.
	#[crate::test]
	fn byte_equality_reopen() {
		let mut source = world();
		let root = spawn_tree(&mut source);
		let first = save(&mut source, root);

		let mut reopened = world();
		let loaded = TemplateLoader::new(&mut reopened)
			.load(&MediaBytes::new_json(first.clone()))
			.unwrap();
		save(&mut reopened, loaded[0]).xpect_eq(first);
	}

	/// An entity added after the load keeps every existing key where it was and
	/// mints a fresh one for itself, so an edit never renumbers the entities it did
	/// not touch, and the new entity lands in file order at its child position.
	#[crate::test]
	fn added_entity_mints_a_fresh_key() {
		let mut source = world();
		let root = spawn_tree(&mut source);
		let first = save(&mut source, root);

		let mut edited = world();
		let loaded = TemplateLoader::new(&mut edited)
			.load(&MediaBytes::new_json(first))
			.unwrap();
		// insert a child at the front, the case a positional scheme gets wrong
		let added = edited.spawn(Name::new("new")).id();
		edited.entity_mut(loaded[0]).insert_children(0, &[added]);

		let text = save(&mut edited, loaded[0]);
		// the loaded entities keep their keys, the new one takes the next unused
		// key, and file order still carries child order
		entity_keys(&edited, &text).xpect_eq(vec![0, 3, 1, 2]);
		let json: serde_json::Value = serde_json::from_str(&text).unwrap();
		json["entities"]["3"]["components"]["bevy_ecs::name::Name"]
			.as_str()
			.unwrap()
			.xpect_eq("new");
	}
}
