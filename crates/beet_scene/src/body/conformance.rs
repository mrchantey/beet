//! Conformance tests for the serialized format's identity guarantees, run
//! against the real [`TemplateSaver`] and [`TemplateLoader`] rather than a
//! hand-written fixture, so they pin what those actually emit. The map itself
//! is [`TemplateEntityMap`].

use crate::prelude::*;
use beet_core::prelude::*;

/// A world that can round-trip a named hierarchy.
fn world() -> World { <(ScenePlugin, MinimalTypesPlugin)>::world() }

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

/// The saved `text` as the [`Value`] a probe indexes.
fn parse(text: &str) -> Value {
	MediaBytes::new_json(text).deserialize::<Value>().unwrap()
}

/// The file keys in **file order**, read back as a [`SceneBody`].
///
/// A `serde_json::Value` would lose that order (its map sorts keys as
/// strings, so `"10"` lands before `"2"`), and order is the children-order
/// contract: the build path applies each `ChildOf` in file order, so file
/// order *is* child order.
fn entity_keys(json: &str) -> Vec<String> {
	MediaBytes::new_json(json)
		.deserialize::<SceneBody>()
		.unwrap()
		.entities
		.keys()
		.map(EntityKey::to_string)
		.collect()
}

/// The observed encoding, straight off the saver: file keys are the minted
/// keys `"0"`, `"1"`, `"2"`, and a `ChildOf` holds the *parent's file key*,
/// not whatever bits it happens to hold this run.
#[beet_core::test]
fn format_probe() {
	let mut world = world();
	let root = spawn_tree(&mut world);
	let scene = parse(&save(&mut world, root));

	let entities = scene
		.get(SceneEntities::ENTITIES)
		.unwrap()
		.as_map()
		.unwrap();
	entities.len().xpect_eq(3);
	for key in ["0", "1", "2"] {
		entities.contains(key).xpect_true();
	}
	// the root carries no parent, each child points at file key 0
	SceneEntities::entity_value(&scene, 0)
		.unwrap()
		.get("bevy_ecs::hierarchy::ChildOf")
		.xpect_none();
	// an `Entity`-typed field is a reference, written as its target's key:
	// bevy's own encoding of one (`Entity::to_bits`, opaque and not
	// guaranteed across versions) never reaches a document.
	for child in [1, 2] {
		SceneEntities::entity_value(&scene, child)
			.unwrap()
			.get("bevy_ecs::hierarchy::ChildOf")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("0");
	}
}

/// A resource set is a map of type path to value with nothing wrapping it,
/// exactly as an entity is.
#[beet_core::test]
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
	TemplateSaver::new()
		.extract_resources()
		.save(&world, MediaType::Json)
		.unwrap()
		.deserialize::<Value>()
		.unwrap()
		.get(SceneEntities::RESOURCES)
		.unwrap()
		.get(Score::type_path())
		.unwrap()
		.as_u64()
		.unwrap()
		.xpect_eq(7);
}

/// Save, load, save: the second save is byte-identical to the first.
///
/// This is the whole point of the retained map. Without it the reload's
/// fresh entity bits would leak into the file and every reopen would produce
/// a different document that happens to mean the same thing.
#[beet_core::test]
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
#[beet_core::test]
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
	SceneEntities::entity_value(&parse(&text), 3)
		.unwrap()
		.get("bevy_ecs::name::Name")
		.unwrap()
		.as_str()
		.unwrap()
		.xpect_eq("new");
}
