//! [`SceneDocument`]: a loaded scene as an editable [`Document`] the world
//! follows.
//!
//! **Edit the document, never the world.** A scene document is the
//! `template_serde` shape (`{ resources, entities }`) held as a [`Value`] on the
//! entity whose children the scene built into. An inspector reads and writes
//! that document; [`sync_scene_documents`] diffs each change against what the
//! world reflects and applies it per component, in place: a changed value
//! reflect-applies into the live component, an added key inserts one, a removed
//! key removes one, a `ChildOf` edit reparents and a removed entity despawns its
//! subtree. The running world never writes back: it is a dump of the scene at
//! runtime, and a dump is never a source.
//!
//! Order rides the document. A `Value` map keeps insertion order, so the
//! document's `entities` are in file order, which is child order; the sync
//! enforces it onto each parent's `Children`, and the fork writes it back
//! through the format's own serializer ([`SceneDocument::to_bytes`]), so a
//! reboot from the store reproduces the edited world.
//!
//! A write that would leave the scene invalid (a cyclic `ChildOf`, a dangling
//! reference, an unregistered component, a value its component rejects) is
//! refused whole: the document is reverted to what the world reflects and the
//! error names the offence, so the world is never half-applied.

use super::apply_value;
use super::serde::TEMPLATE_RESOURCES;
use super::serde::TypedValueDeserializer;
use super::write_resource;
use crate::prelude::*;
use bevy::ecs::entity::EntityMapper;
use bevy::ecs::reflect::ReflectComponent;
use bevy_reflect::PartialReflect;
use bevy_reflect::TypeRegistry;
use serde::Serialize;
use serde::de::DeserializeSeed;

/// The scene this entity's children were loaded from, as an editable
/// [`Document`] the world follows.
///
/// Landed by [`load`](Self::load) (boot from a scene) or [`fork`](Self::fork)
/// (turn what is built into a scene) beside the [`Document`], its
/// [`DocumentSchema`] ([`ValueSchema::SCENE`]) and the [`TemplateEntityMap`]
/// linking file keys to the live entities. Every edit to the document reaches
/// the world through [`sync_scene_documents`].
#[derive(Component)]
pub struct SceneDocument {
	/// What the world currently reflects: the base the next edit is diffed
	/// against, and what the document reverts to when an edit is refused.
	synced: Value,
}

/// Records the authored original a scene was forked from, on the scene's first
/// entity, so the fork knows its origin wherever the store travels.
///
/// The fork relation, Automerge-shaped: today the whole scene is the fork and
/// the .bsx stays the untouched original; a merge policy (a patch overlay over
/// an updating original) grows here without changing the format around it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Component, Reflect)]
#[reflect(Component)]
pub struct SceneFork {
	/// The store path of the authored original.
	pub from: SmolPath,
}

impl SceneFork {
	/// A scene forked from the authored original at `from`.
	pub fn new(from: impl Into<SmolPath>) -> Self { Self { from: from.into() } }
}

impl SceneDocument {
	/// The registered `ChildOf`, the relation the sync keeps in document order.
	const CHILD_OF: &'static str = "bevy_ecs::hierarchy::ChildOf";

	/// Build `template` as `host`'s children and land its document on `host`.
	///
	/// The document and the world come from the one parsed template, so they
	/// start in agreement: the document is the template as a [`Value`], the
	/// world its build, the [`TemplateEntityMap`] the keys that build handed
	/// out. Returns every spawned entity.
	pub fn load(
		world: &mut World,
		host: Entity,
		template: DynamicTemplate,
	) -> Result<Vec<Entity>> {
		let value = Self::template_value(world, &template)?;
		let (spawned, entity_map) = TemplateLoader::build(
			world,
			Some(host),
			EntryTemplate::Serde(template),
		)?;
		Self::land(world, host, value, entity_map);
		Ok(spawned)
	}

	/// [`load`](Self::load) from serialized bytes, ie the stored fork.
	pub fn load_bytes(
		world: &mut World,
		host: Entity,
		bytes: &MediaBytes,
	) -> Result<Vec<Entity>> {
		match EntryTemplate::from_bytes(world, bytes)? {
			EntryTemplate::Serde(template) => Self::load(world, host, template),
			#[cfg(feature = "bsx")]
			EntryTemplate::Bsx(_) => bevybail!(
				"a scene is a serde document, not markup: build the `{}` \
				original first and fork it",
				bytes.media_type()
			),
		}
	}

	/// Fork: serialize `host`'s children as one scene, land its document on
	/// `host` and return the bytes for the store.
	///
	/// The first boot of an authored original: whatever built the children (a
	/// `.bsx` entry, a spawned bundle) stays live, and the scene it dumps to is
	/// the boot source from now on. The document is read back from the bytes
	/// rather than taken from the extractor, so a first boot and a boot from the
	/// stored fork hold the identical value (a float's text form, for one, is
	/// what the file says, not what the extractor held).
	pub fn fork(
		world: &mut World,
		host: Entity,
		media_type: MediaType,
	) -> Result<MediaBytes> {
		let roots = world
			.entity(host)
			.get::<Children>()
			.map(|children| children.iter().collect::<Vec<_>>())
			.unwrap_or_default();
		let (bytes, entity_map) = TemplateSaver::new().save_roots_mapped(
			world,
			media_type,
			&roots,
			default(),
		)?;
		let value = match EntryTemplate::from_bytes(world, &bytes)? {
			EntryTemplate::Serde(template) => {
				Self::template_value(world, &template)?
			}
			#[cfg(feature = "bsx")]
			EntryTemplate::Bsx(_) => unreachable!("a saver writes serde"),
		};
		Self::land(world, host, value, entity_map);
		Ok(bytes)
	}

	/// Serialize a scene document's value as the format writes it: entities in
	/// document order, component and resource maps sorted by type path, so an
	/// unedited document re-saves byte-identical to the fork it was read from.
	///
	/// The document is first read into a [`DynamicTemplate`], which is also what
	/// proves it persistable: a value its component's type rejects fails here
	/// rather than at the next boot.
	pub fn to_bytes(
		registry: &TypeRegistry,
		value: &Value,
		media_type: MediaType,
	) -> Result<MediaBytes> {
		let template = Self::value_template(registry, value)?;
		MediaBytes::serialize(
			media_type,
			&DynamicTemplateSerializer::new(&template, registry),
		)
	}

	/// The document components a scene lands on its host.
	fn land(
		world: &mut World,
		host: Entity,
		value: Value,
		entity_map: TemplateEntityMap,
	) {
		world.entity_mut(host).insert((
			Document::new(value.clone()),
			DocumentSchema(ValueSchema::reference(ValueSchema::SCENE)),
			Self { synced: value },
			entity_map,
		));
	}

	/// A template as the [`Value`] its serialized form reads back to.
	fn template_value(
		world: &World,
		template: &DynamicTemplate,
	) -> Result<Value> {
		let registry = world.resource::<AppTypeRegistry>().read();
		DynamicTemplateSerializer::new(template, &registry)
			.serialize(ValueSerializer)?
			.xok()
	}

	/// A scene document's value as the template it describes.
	fn value_template(
		registry: &TypeRegistry,
		value: &Value,
	) -> Result<DynamicTemplate> {
		DynamicTemplateDeserializer {
			type_registry: registry,
		}
		.deserialize(ValueDeserializer::new(value.clone()))?
		.xok()
	}

	/// Whether `next` differs from what the world reflects.
	///
	/// Map equality ignores order, and for a scene order is meaning (the
	/// entities' order is child order), so a pure reorder counts as a change.
	fn changed(&self, next: &Value) -> bool {
		let keys = |scene: &Value| {
			SceneEntities::of(scene)
				.and_then(|entities| entities.keys())
				.ok()
		};
		self.synced != *next || keys(&self.synced) != keys(next)
	}

	/// Bring the world to `next`: plan every change against what it reflects,
	/// refusing the whole edit if any part is invalid, then commit.
	fn apply(world: &mut World, host: Entity, next: Value) -> Result {
		let scene = world
			.get::<Self>(host)
			.ok_or_else(|| bevyhow!("entity {host} holds no scene document"))?;
		if !scene.changed(&next) {
			return OK;
		}
		let synced = scene.synced.clone();
		let app_registry = world.resource::<AppTypeRegistry>().clone();
		let registry = app_registry.read();
		match ScenePlan::new(&registry, &synced, &next) {
			Ok(plan) => {
				plan.commit(world, host, &registry, &next);
				world.get_mut::<Self>(host).unwrap().synced = next;
				OK
			}
			Err(err) => {
				// refuse the write: the document goes back to what the world
				// reflects, so the two never disagree
				world.get_mut::<Document>(host).unwrap().0 = synced;
				bevybail!("scene edit refused, document reverted: {err}")
			}
		}
	}
}

/// The system driving every changed scene document into its world, in the
/// [`DocumentSync`] chain after the field write-back, so an inspector's edit
/// lands in the world in the same pass it lands in the document.
pub(crate) fn sync_scene_documents(world: &mut World) -> Result {
	let changed = world
		.query_filtered::<(Entity, &Document, &SceneDocument), Changed<Document>>(
		)
		.iter(world)
		// the revert of a refused edit is itself a change, already reflected
		.filter(|(_, document, scene)| scene.changed(&document.0))
		.map(|(host, document, _)| (host, document.0.clone()))
		.collect::<Vec<_>>();
	for (host, next) in changed {
		SceneDocument::apply(world, host, next)?;
	}
	OK
}

/// Every change one edit asks of the world, validated and deserialized before
/// anything is touched, so a refused edit leaves the world exactly as it was.
struct ScenePlan {
	/// Entities the document dropped, whose subtrees despawn.
	removed: Vec<u32>,
	/// Every entity the document holds, in document order.
	entities: Vec<EntityPlan>,
	/// Resource values to write.
	resources: Vec<Box<dyn PartialReflect>>,
	/// Resource type paths to remove.
	removed_resources: Vec<SmolStr>,
}

/// One entity's share of a [`ScenePlan`].
struct EntityPlan {
	key: u32,
	/// Whether the document holds a `ChildOf` for it; without one it is a root
	/// and sits under the host.
	has_parent: bool,
	/// Component values to apply, in document order.
	apply: Vec<Box<dyn PartialReflect>>,
	/// Component type paths to remove.
	remove: Vec<SmolStr>,
}

impl ScenePlan {
	/// Diff `next` against `synced`, deserializing every changed value.
	fn new(
		registry: &TypeRegistry,
		synced: &Value,
		next: &Value,
	) -> Result<Self> {
		let old = SceneEntities::of(synced)?;
		let new = SceneEntities::of(next)?;
		// the whole-document check the walk cannot make: the world would only
		// discover a cycle by panicking
		new.assert_acyclic(registry)?;

		let removed = old
			.keys()?
			.into_iter()
			.filter(|key| !new.contains(*key))
			.collect();

		let mut references = Vec::new();
		let mut entities = Vec::new();
		for key in new.keys()? {
			let components = new.components(key).ok_or_else(|| {
				bevyhow!("entity #{key} holds no `components` map")
			})?;
			let previous = old.components(key);
			let mut plan = EntityPlan {
				key,
				has_parent: components.contains(SceneDocument::CHILD_OF),
				apply: Vec::new(),
				remove: Vec::new(),
			};
			for (type_path, value) in components.iter() {
				if previous.and_then(|map| map.0.get(type_path)) == Some(value)
				{
					continue;
				}
				let mut component =
					deserialize_typed(registry, type_path, value)
						.map_err(|err| bevyhow!("entity #{key}: {err}"))?;
				collect_references(
					registry,
					&mut component,
					key,
					&mut references,
				);
				plan.apply.push(component);
			}
			plan.remove.extend(
				previous
					.into_iter()
					.flat_map(|map| map.0.keys())
					.filter(|type_path| !components.contains(type_path))
					.cloned(),
			);
			entities.push(plan);
		}
		// a reference must name an entity the document holds: the world would
		// otherwise gain an entity the document does not describe
		for (source, target) in references {
			if !new.contains(target) {
				bevybail!(
					"entity #{source} references #{target}, which the scene \
					does not hold"
				);
			}
		}

		let (resources, removed_resources) =
			Self::resource_diff(registry, synced, next)?;
		Self {
			removed,
			entities,
			resources,
			removed_resources,
		}
		.xok()
	}

	/// The resources to write and to remove.
	fn resource_diff(
		registry: &TypeRegistry,
		synced: &Value,
		next: &Value,
	) -> Result<(Vec<Box<dyn PartialReflect>>, Vec<SmolStr>)> {
		let resources = |scene: &Value| -> Result<Map> {
			match scene.get(TEMPLATE_RESOURCES) {
				Some(resources) => resources.as_map()?.clone().xok(),
				None => Map::default().xok(),
			}
		};
		let (old, new) = (resources(synced)?, resources(next)?);
		let write = new
			.iter()
			.filter(|(type_path, value)| old.0.get(*type_path) != Some(value))
			.map(|(type_path, value)| {
				deserialize_typed(registry, type_path, value)
					.map_err(|err| bevyhow!("resource `{type_path}`: {err}"))
			})
			.collect::<Result<Vec<_>>>()?;
		let remove = old
			.0
			.keys()
			.filter(|type_path| !new.contains(type_path))
			.cloned()
			.collect();
		Ok((write, remove))
	}

	/// Apply the plan, bringing the world to `next`. Infallible by
	/// construction: every lookup it needs was made while planning.
	fn commit(
		self,
		world: &mut World,
		host: Entity,
		registry: &TypeRegistry,
		next: &Value,
	) {
		// removed entities despawn with their subtrees, which the plan proved
		// the document no longer references
		for key in self.removed {
			let entity = world
				.get_mut::<TemplateEntityMap>(host)
				.unwrap()
				.remove(key);
			if let Some(entity) = entity
				&& let Ok(entity) = world.get_entity_mut(entity)
			{
				entity.despawn();
			}
		}
		// every entity the document holds is live before any reference to it is
		// applied, so a forward reference resolves to the real entity
		for plan in &self.entities {
			let mapped = world
				.get::<TemplateEntityMap>(host)
				.unwrap()
				.world(plan.key)
				.filter(|entity| world.entities().contains(*entity));
			if mapped.is_none() {
				let entity = world.spawn_empty().id();
				world
					.get_mut::<TemplateEntityMap>(host)
					.unwrap()
					.insert(plan.key, entity);
			}
		}
		let entity_map = world.get::<TemplateEntityMap>(host).unwrap().clone();
		let mut mapper = KeyMapper(&entity_map);
		for plan in &self.entities {
			let entity = entity_map.world(plan.key).unwrap();
			for component in &plan.apply {
				apply_value(
					world,
					entity,
					component.as_ref(),
					&mut mapper,
					registry,
				)
				.expect("planned against this registry");
			}
			for type_path in &plan.remove {
				if let Some(reflect) = reflect_component(registry, type_path) {
					reflect.remove(&mut world.entity_mut(entity));
				}
			}
			// a root sits under the host, which is not a scene entity
			if !plan.has_parent
				&& world.get::<ChildOf>(entity).map(ChildOf::parent)
					!= Some(host)
			{
				world.entity_mut(entity).insert(ChildOf(host));
			}
		}
		for resource in &self.resources {
			write_resource(world, resource.as_ref(), &mut mapper, registry)
				.expect("planned against this registry");
		}
		for type_path in &self.removed_resources {
			remove_resource(world, registry, type_path);
		}
		// the document's entity order is child order: each parent's `Children`
		// follow it, so a reparent lands where the document says and a reboot
		// from the store reproduces the live order
		let entities = SceneEntities::of(next).expect("planned");
		let mut children_of = HashMap::<u32, Vec<Entity>>::default();
		for key in entities.keys().expect("planned") {
			if let Some(parent) = entities.target(key, SceneDocument::CHILD_OF)
			{
				children_of
					.entry(parent)
					.or_default()
					.push(entity_map.world(key).unwrap());
			}
		}
		for (parent, ordered) in children_of {
			order_children(world, entity_map.world(parent).unwrap(), &ordered);
		}
	}
}

/// Put the document's children of `parent` in `ordered` order, leaving any
/// child the document does not describe (an editor's furniture, a form's rows)
/// exactly where it sits.
fn order_children(world: &mut World, parent: Entity, ordered: &[Entity]) {
	let children = world
		.get::<Children>(parent)
		.map(|children| children.iter().collect::<Vec<_>>())
		.unwrap_or_default();
	// the described children still under this parent, and the slots they
	// occupy, refilled in document order
	let present = ordered
		.iter()
		.filter(|child| children.contains(child))
		.copied()
		.collect::<HashSet<_>>();
	let slots = children
		.iter()
		.enumerate()
		.filter(|(_, child)| present.contains(*child))
		.map(|(slot, _)| slot);
	let reordered = slots
		.zip(ordered.iter().filter(|child| present.contains(*child)))
		.fold(children.clone(), |mut children, (slot, child)| {
			children[slot] = *child;
			children
		});
	if reordered != children {
		world.entity_mut(parent).replace_children(&reordered);
	}
}

/// Maps a file-key entity (`Entity::from_raw_u32(key)`, as a document writes a
/// reference) to the live entity the document's [`TemplateEntityMap`] holds
/// for it. Every key was proved present and made live before this runs.
struct KeyMapper<'a>(&'a TemplateEntityMap);

impl EntityMapper for KeyMapper<'_> {
	fn get_mapped(&mut self, source: Entity) -> Entity {
		if source == Entity::PLACEHOLDER {
			return Entity::PLACEHOLDER;
		}
		self.0
			.world(source.index_u32())
			.unwrap_or(Entity::PLACEHOLDER)
	}
	fn set_mapped(&mut self, _source: Entity, _target: Entity) {}
}

/// Records every entity a component references, mapping nothing.
struct ReferenceCollector<'a> {
	source: u32,
	references: &'a mut Vec<(u32, u32)>,
}

impl EntityMapper for ReferenceCollector<'_> {
	fn get_mapped(&mut self, entity: Entity) -> Entity {
		// a placeholder names no entity
		if entity != Entity::PLACEHOLDER {
			self.references.push((self.source, entity.index_u32()));
		}
		entity
	}
	fn set_mapped(&mut self, _source: Entity, _target: Entity) {}
}

/// Every `(referrer, target)` file-key pair `component` holds, via the
/// registered `map_entities`, so a dangling reference is caught before the
/// world sees it. The collector maps every entity to itself, so the component
/// is unchanged by the walk.
fn collect_references(
	registry: &TypeRegistry,
	component: &mut Box<dyn PartialReflect>,
	source: u32,
	references: &mut Vec<(u32, u32)>,
) {
	let Some(reflect) = component
		.get_represented_type_info()
		.and_then(|info| registry.get(info.type_id()))
		.and_then(|registration| registration.data::<ReflectComponent>())
	else {
		return;
	};
	// a concrete value (the typed reader converts through `FromReflect`) is
	// `Reflect`; a dynamic that failed to convert holds no mappable entity
	if let Some(concrete) = component.try_as_reflect_mut() {
		reflect.map_entities(concrete, &mut ReferenceCollector {
			source,
			references,
		});
	}
}

/// Read a component or resource value as the registered type at `type_path`.
fn deserialize_typed(
	registry: &TypeRegistry,
	type_path: &str,
	value: &Value,
) -> Result<Box<dyn PartialReflect>> {
	let registration = registry
		.get_with_type_path(type_path)
		.ok_or_else(|| bevyhow!("`{type_path}` is not a registered type"))?;
	TypedValueDeserializer {
		registration,
		registry,
	}
	.deserialize(ValueDeserializer::new(value.clone()))
	.map_err(|err| bevyhow!("`{type_path}`: {err}"))
}

/// The reflected component registered at `type_path`, if any.
fn reflect_component<'a>(
	registry: &'a TypeRegistry,
	type_path: &str,
) -> Option<&'a ReflectComponent> {
	registry
		.get_with_type_path(type_path)?
		.data::<ReflectComponent>()
}

/// Remove the resource registered at `type_path` from the world, if present.
fn remove_resource(
	world: &mut World,
	registry: &TypeRegistry,
	type_path: &str,
) {
	let Some(reflect) = reflect_component(registry, type_path) else {
		return;
	};
	let id = reflect.register_component(world);
	if let Some(entity) = world.resource_entities().get(id) {
		reflect.remove(&mut world.entity_mut(entity));
	}
}

#[cfg(all(test, feature = "json"))]
mod test {
	use crate::prelude::*;

	/// A component with runtime state the document never sees, so an untouched
	/// sibling proves the apply is in place.
	#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
	#[reflect(Component)]
	struct Health(u32);

	/// Runtime-only state: unregistered, so it is neither dumped nor applied.
	#[derive(Component)]
	struct Ticks(u32);

	#[derive(Debug, Default, Clone, PartialEq, Resource, Reflect)]
	#[reflect(Resource)]
	struct Score(u32);

	const HEALTH: &str =
		"beet_core::template_serde::scene_document::test::Health";
	const SCORE: &str =
		"beet_core::template_serde::scene_document::test::Score";
	const NAME: &str = "bevy_ecs::name::Name";
	const CHILD_OF: &str = "bevy_ecs::hierarchy::ChildOf";

	fn new_world() -> World {
		let world = <(DocumentPlugin, MinimalTypesPlugin)>::world();
		world
			.resource::<AppTypeRegistry>()
			.write()
			.register::<Health>();
		world
			.resource::<AppTypeRegistry>()
			.write()
			.register::<Score>();
		world
	}

	/// A host whose child tree is the authored original, forked into a scene
	/// document: `root { a(Health 1), b(Health 2) }` as keys `0, 1, 2`.
	fn forked() -> (World, Entity, MediaBytes) {
		let mut world = new_world();
		let host = world
			.spawn(children![(Name::new("root"), children![
				(Name::new("a"), Health(1), Ticks(7)),
				(Name::new("b"), Health(2)),
			])])
			.flush();
		let bytes =
			SceneDocument::fork(&mut world, host, MediaType::Json).unwrap();
		(world, host, bytes)
	}

	/// The live entity at `key`.
	fn entity(world: &World, host: Entity, key: u32) -> Entity {
		world
			.get::<TemplateEntityMap>(host)
			.unwrap()
			.world(key)
			.unwrap()
	}

	fn names(world: &World, parent: Entity) -> Vec<String> {
		world
			.get::<Children>(parent)
			.map(|children| {
				children
					.iter()
					.map(|child| world.get::<Name>(child).unwrap().to_string())
					.collect()
			})
			.unwrap_or_default()
	}

	/// Edit the scene document as an inspector would, through `DocumentQuery`,
	/// then run one sync pass.
	fn edit(
		world: &mut World,
		host: Entity,
		func: impl FnOnce(&mut Value) + Send + Sync + 'static,
	) {
		world
			.run_system_once_with(
				|In((host, func)): In<(
					Entity,
					Box<dyn FnOnce(&mut Value) + Send + Sync>,
				)>,
				 mut query: DocumentQuery| {
					// the whole document, on the host itself
					let field = FieldRef::new(FieldPath::default())
						.with_document(DocumentPath::This);
					query
						.with_field(host, &field, |value| func(value))
						.unwrap();
				},
				(host, Box::new(func) as Box<_>),
			)
			.unwrap();
		world.run_schedule(DocumentSync);
	}

	/// A component's document slot.
	fn component<'a>(
		scene: &'a mut Value,
		key: u32,
		type_path: &str,
	) -> &'a mut Value {
		scene
			.get_mut("entities")
			.unwrap()
			.get_mut(&key.to_string())
			.unwrap()
			.get_mut("components")
			.unwrap()
			.as_map_mut()
			.unwrap()
			.entry(type_path.into())
			.or_insert(Value::Null)
	}

	#[crate::test]
	fn a_fork_lands_the_document_beside_the_world() {
		let (world, host, _) = forked();
		let scene =
			SceneEntities::of(&world.get::<Document>(host).unwrap().0).unwrap();
		scene.keys().unwrap().xpect_eq(vec![0, 1, 2]);
		scene.related(CHILD_OF, 0).unwrap().xpect_eq(vec![1, 2]);
		world
			.get::<DocumentSchema>(host)
			.unwrap()
			.0
			.clone()
			.xpect_eq(ValueSchema::reference(ValueSchema::SCENE));
		// the live tree is untouched by the fork, and keyed
		world
			.get::<Name>(entity(&world, host, 1))
			.unwrap()
			.as_str()
			.xpect_eq("a");
		world
			.get::<ChildOf>(entity(&world, host, 0))
			.unwrap()
			.parent()
			.xpect_eq(host);
	}

	/// A value change reflect-applies into the live component: the entity, its
	/// other components and its sibling's runtime state all survive.
	#[crate::test]
	fn a_value_change_applies_in_place() {
		let (mut world, host, _) = forked();
		let (a, b) = (entity(&world, host, 1), entity(&world, host, 2));
		edit(&mut world, host, |scene| {
			*component(scene, 2, HEALTH) = Value::Uint(9);
		});
		world.get::<Health>(b).unwrap().xpect_eq(Health(9));
		world.get::<Health>(a).unwrap().xpect_eq(Health(1));
		world.get::<Ticks>(a).unwrap().0.xpect_eq(7);
		// the same entities, not respawned ones
		entity(&world, host, 1).xpect_eq(a);
		entity(&world, host, 2).xpect_eq(b);
	}

	#[crate::test]
	fn a_component_is_added_and_removed() {
		let (mut world, host, _) = forked();
		let root = entity(&world, host, 0);
		edit(&mut world, host, |scene| {
			*component(scene, 0, HEALTH) = Value::Uint(3);
		});
		world.get::<Health>(root).unwrap().xpect_eq(Health(3));
		edit(&mut world, host, |scene| {
			component(scene, 0, HEALTH);
			scene
				.get_mut("entities")
				.unwrap()
				.get_mut("0")
				.unwrap()
				.get_mut("components")
				.unwrap()
				.as_map_mut()
				.unwrap()
				.remove(HEALTH);
		});
		world.get::<Health>(root).xpect_none();
		world.get::<Name>(root).unwrap().as_str().xpect_eq("root");
	}

	/// A `ChildOf` edit reparents in place, landing at the position the
	/// document's entity order gives it among its new siblings.
	#[crate::test]
	fn a_reparent_follows_document_order() {
		let (mut world, host, _) = forked();
		let (root, a, b) = (
			entity(&world, host, 0),
			entity(&world, host, 1),
			entity(&world, host, 2),
		);
		// `b` under `a`
		edit(&mut world, host, |scene| {
			*component(scene, 2, CHILD_OF) =
				EntitySchema::reference(1).unwrap();
		});
		names(&world, root).xpect_eq(vec!["a".to_string()]);
		names(&world, a).xpect_eq(vec!["b".to_string()]);
		world.get::<ChildOf>(b).unwrap().parent().xpect_eq(a);
		// back under the root: `b` precedes `a` in the world only if the
		// document says so, and the document says `a` first
		edit(&mut world, host, |scene| {
			*component(scene, 2, CHILD_OF) =
				EntitySchema::reference(0).unwrap();
		});
		names(&world, root).xpect_eq(vec!["a".to_string(), "b".into()]);
		// dropping the `ChildOf` makes it a root under the host
		edit(&mut world, host, |scene| {
			scene
				.get_mut("entities")
				.unwrap()
				.get_mut("2")
				.unwrap()
				.get_mut("components")
				.unwrap()
				.as_map_mut()
				.unwrap()
				.remove(CHILD_OF);
		});
		world.get::<ChildOf>(b).unwrap().parent().xpect_eq(host);
		names(&world, root).xpect_eq(vec!["a".to_string()]);
	}

	/// Moving an entity within the document reorders its parent's children,
	/// around any child the document does not describe: runtime furniture (an
	/// editor's widgets, a form's rows) keeps its slot and its parent.
	#[crate::test]
	fn a_reorder_follows_the_document() {
		let (mut world, host, _) = forked();
		let root = entity(&world, host, 0);
		let furniture =
			world.spawn((Name::new("furniture"), ChildOf(root))).id();
		world.entity_mut(root).insert_children(1, &[furniture]);
		names(&world, root).xpect_eq(vec![
			"a".to_string(),
			"furniture".into(),
			"b".into(),
		]);
		edit(&mut world, host, |scene| {
			let entities =
				scene.get_mut("entities").unwrap().as_map_mut().unwrap();
			entities.0.move_index(2, 1);
		});
		names(&world, root).xpect_eq(vec![
			"b".to_string(),
			"furniture".into(),
			"a".into(),
		]);
	}

	#[crate::test]
	fn a_removed_entity_despawns_its_subtree() {
		let (mut world, host, _) = forked();
		let (root, a, b) = (
			entity(&world, host, 0),
			entity(&world, host, 1),
			entity(&world, host, 2),
		);
		edit(&mut world, host, |scene| {
			*component(scene, 2, CHILD_OF) =
				EntitySchema::reference(1).unwrap();
		});
		// removing `a` takes `b` with it, so the document drops both
		edit(&mut world, host, |scene| {
			let entities =
				scene.get_mut("entities").unwrap().as_map_mut().unwrap();
			entities.remove("1");
			entities.remove("2");
		});
		world.get_entity(a).is_err().xpect_true();
		world.get_entity(b).is_err().xpect_true();
		names(&world, root).xpect_eq(Vec::<String>::new());
		world
			.get::<TemplateEntityMap>(host)
			.unwrap()
			.world(1)
			.xpect_none();
	}

	/// A new entity spawns with its components, under the parent it names,
	/// and its fresh key is retained.
	#[crate::test]
	fn an_added_entity_spawns() {
		let (mut world, host, _) = forked();
		let a = entity(&world, host, 1);
		edit(&mut world, host, |scene| {
			scene
				.get_mut("entities")
				.unwrap()
				.insert(
					"3",
					value!({ "components": {
					(NAME): "c",
					(CHILD_OF): (EntitySchema::reference(1).unwrap())
				} }),
				)
				.unwrap();
		});
		let c = entity(&world, host, 3);
		world.get::<Name>(c).unwrap().as_str().xpect_eq("c");
		world.get::<ChildOf>(c).unwrap().parent().xpect_eq(a);
		// a root without a parent lands under the host
		edit(&mut world, host, |scene| {
			scene
				.get_mut("entities")
				.unwrap()
				.insert("4", value!({ "components": { (NAME): "d" } }))
				.unwrap();
		});
		let d = entity(&world, host, 4);
		world.get::<ChildOf>(d).unwrap().parent().xpect_eq(host);
	}

	#[crate::test]
	fn a_resource_is_written_and_removed() {
		let (mut world, host, _) = forked();
		edit(&mut world, host, |scene| {
			scene
				.get_mut("resources")
				.unwrap()
				.insert(SCORE, Value::Uint(5))
				.unwrap();
		});
		world.resource::<Score>().xpect_eq(Score(5));
		edit(&mut world, host, |scene| {
			scene
				.get_mut("resources")
				.unwrap()
				.as_map_mut()
				.unwrap()
				.remove(SCORE);
		});
		world.get_resource::<Score>().xpect_none();
	}

	/// A write the scene cannot hold is refused whole: the error names it, the
	/// document reverts to what the world reflects, the world is untouched.
	#[crate::test]
	fn an_invalid_edit_is_refused_and_reverted() {
		let (mut world, host, _) = forked();
		let before = world.get::<Document>(host).unwrap().0.clone();
		let refuse = |world: &mut World, func: fn(&mut Value)| -> String {
			let mut scene = world.get::<Document>(host).unwrap().0.clone();
			func(&mut scene);
			world.get_mut::<Document>(host).unwrap().0 = scene;
			let err: Result =
				world.run_system_once(sync_scene_documents).unwrap();
			let err = err.unwrap_err().to_string();
			world
				.get::<Document>(host)
				.unwrap()
				.0
				.clone()
				.xpect_eq(before.clone());
			err
		};
		// the root under its own child cycles
		refuse(&mut world, |scene| {
			*component(scene, 0, CHILD_OF) =
				EntitySchema::reference(2).unwrap();
		})
		.xpect_contains("would cycle")
		.xpect_contains("#0")
		.xpect_contains("#2");
		// a parent the scene does not hold dangles
		refuse(&mut world, |scene| {
			*component(scene, 2, CHILD_OF) =
				EntitySchema::reference(9).unwrap();
		})
		.xpect_contains("#2 references #9");
		// a component this binary has not registered, and a value its type
		// rejects, both fail before anything is applied
		refuse(&mut world, |scene| {
			*component(scene, 2, "made::Up") = Value::Uint(1);
		})
		.xpect_contains("made::Up")
		.xpect_contains("not a registered type");
		refuse(&mut world, |scene| {
			*component(scene, 1, HEALTH) = Value::Str("full".into());
		})
		.xpect_contains("#1")
		.xpect_contains(HEALTH);
		world
			.get::<Health>(entity(&world, host, 1))
			.unwrap()
			.xpect_eq(Health(1));
		world
			.get::<ChildOf>(entity(&world, host, 2))
			.unwrap()
			.parent()
			.xpect_eq(entity(&world, host, 0));
	}

	/// A reboot from the fork reproduces the edited world: same names, same
	/// order, same values, keys intact.
	#[crate::test]
	fn a_reboot_reproduces_the_edited_world() {
		let (mut world, host, _) = forked();
		edit(&mut world, host, |scene| {
			*component(scene, 2, HEALTH) = Value::Uint(9);
			*component(scene, 2, CHILD_OF) =
				EntitySchema::reference(1).unwrap();
			scene
				.get_mut("entities")
				.unwrap()
				.insert(
					"3",
					value!({ "components": {
					(NAME): "c",
					(CHILD_OF): (EntitySchema::reference(0).unwrap())
				} }),
				)
				.unwrap();
		});
		let registry = world.resource::<AppTypeRegistry>().read();
		let bytes = SceneDocument::to_bytes(
			&registry,
			&world.get::<Document>(host).unwrap().0,
			MediaType::Json,
		)
		.unwrap();
		drop(registry);

		let mut rebooted = new_world();
		let host = rebooted.spawn_empty().id();
		SceneDocument::load_bytes(&mut rebooted, host, &bytes).unwrap();
		let root = entity(&rebooted, host, 0);
		names(&rebooted, root).xpect_eq(vec!["a".to_string(), "c".into()]);
		names(&rebooted, entity(&rebooted, host, 1))
			.xpect_eq(vec!["b".to_string()]);
		rebooted
			.get::<Health>(entity(&rebooted, host, 2))
			.unwrap()
			.xpect_eq(Health(9));
		// and the rebooted document is the one that was written
		rebooted
			.get::<Document>(host)
			.unwrap()
			.0
			.clone()
			.xpect_eq(world.get::<Document>(host).unwrap().0.clone());
	}
}

/// Conformance for the fork's identity guarantees, in the style of
/// `entity_map.rs`: what the saver emits, the document re-emits byte for byte.
#[cfg(all(test, feature = "json"))]
mod conformance {
	use crate::prelude::*;

	fn new_world() -> World { <(DocumentPlugin, MinimalTypesPlugin)>::world() }

	/// A host whose children are a named tree.
	fn spawn_host(world: &mut World) -> Entity {
		world
			.spawn(children![(Name::new("parent"), children![
				Name::new("a"),
				Name::new("b"),
			])])
			.flush()
	}

	fn to_bytes(world: &World, host: Entity) -> String {
		let registry = world.resource::<AppTypeRegistry>().read();
		SceneDocument::to_bytes(
			&registry,
			&world.get::<Document>(host).unwrap().0,
			MediaType::Json,
		)
		.unwrap()
		.as_utf8()
		.unwrap()
		.to_string()
	}

	/// Fork, then write the document: byte-identical to the fork. Boot from
	/// those bytes, write again: still identical. The document is the file.
	#[crate::test]
	fn byte_equality_through_the_document() {
		let mut world = new_world();
		let host = spawn_host(&mut world);
		let fork = SceneDocument::fork(&mut world, host, MediaType::Json)
			.unwrap()
			.as_utf8()
			.unwrap()
			.to_string();
		to_bytes(&world, host).xpect_eq(fork.clone());

		let mut rebooted = new_world();
		let host = rebooted.spawn_empty().id();
		SceneDocument::load_bytes(
			&mut rebooted,
			host,
			&MediaBytes::new_json(fork.clone()),
		)
		.unwrap();
		to_bytes(&rebooted, host).xpect_eq(fork);
	}

	/// The fork carries no host: the host's own components never enter the
	/// scene, and the roots carry no `ChildOf` to it.
	#[crate::test]
	fn the_host_is_not_in_the_scene() {
		let mut world = new_world();
		let host = spawn_host(&mut world);
		world.entity_mut(host).insert(Name::new("host"));
		let json = SceneDocument::fork(&mut world, host, MediaType::Json)
			.unwrap()
			.as_utf8()
			.unwrap()
			.xmap(serde_json::from_str::<serde_json::Value>)
			.unwrap();
		let entities = json["entities"].as_object().unwrap();
		entities.len().xpect_eq(3);
		entities["0"]["components"]
			.get("bevy_ecs::hierarchy::ChildOf")
			.xpect_none();
		entities["0"]["components"]["bevy_ecs::name::Name"]
			.as_str()
			.unwrap()
			.xpect_eq("parent");
		// the live root is still the host's child
		let root = world
			.get::<TemplateEntityMap>(host)
			.unwrap()
			.world(0)
			.unwrap();
		world.get::<ChildOf>(root).unwrap().parent().xpect_eq(host);
	}
}
