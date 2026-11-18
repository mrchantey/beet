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

	/// The file keys in **file order**, read back as a [`SceneBody`].
	///
	/// A `serde_json::Value` would lose that order (its map sorts keys as
	/// strings, so `"10"` lands before `"2"`), and order is the children-order
	/// contract: the build path applies each `ChildOf` in file order, so file
	/// order *is* child order.
	fn entity_keys(json: &str) -> Vec<String> {
		SceneBody::from_bytes(&MediaBytes::new_json(json))
			.unwrap()
			.entities
			.keys()
			.map(EntityKey::to_string)
			.collect()
	}

	/// The observed encoding, straight off the saver: file keys are the minted
	/// keys `"0"`, `"1"`, `"2"`, and a `ChildOf` holds the *parent's file key*,
	/// not whatever bits it happens to hold this run.
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
		SceneEntities::entity_json(&json, 0)
			.get("bevy_ecs::hierarchy::ChildOf")
			.xpect_none();
		// an `Entity`-typed field is a reference, written as its target's key:
		// bevy's own encoding of one (`Entity::to_bits`, opaque and not
		// guaranteed across versions) never reaches a document.
		for child in [1, 2] {
			SceneEntities::entity_json(&json, child)
				["bevy_ecs::hierarchy::ChildOf"]
				.as_str()
				.unwrap()
				.xpect_eq("0");
		}
	}

	/// A resource set is a map of type path to value with nothing wrapping it,
	/// exactly as an entity is.
	#[crate::test]
	fn resources_are_flat() {
		#[derive(Resource, Reflect)]
		#[reflect(Resource)]
		struct Score(u32);

		let mut world = world();
		world
			.resource::<AppTypeRegistry>()
			.write()
			.register::<Score>();
		world.insert_resource(Score(7));
		let json = TemplateSaver::new()
			.extract_resources()
			.save(&world, MediaType::Json)
			.unwrap()
			.as_utf8()
			.unwrap()
			.xmap(serde_json::from_str::<serde_json::Value>)
			.unwrap();
		json["resources"][Score::type_path()]
			.as_u64()
			.unwrap()
			.xpect_eq(7);
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
		entity_keys(&text).xpect_eq(vec!["0", "3", "1", "2"]);
		let json: serde_json::Value = serde_json::from_str(&text).unwrap();
		SceneEntities::entity_json(&json, 3)["bevy_ecs::name::Name"]
			.as_str()
			.unwrap()
			.xpect_eq("new");
	}
}
