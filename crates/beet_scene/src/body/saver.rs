use super::TemplateBuilder;
use super::TemplateFilter;
#[cfg(doc)]
use super::TemplateLoader;
use beet_core::prelude::*;
use bevy::ecs::query::QueryFilter;

/// Configures and runs a serialization of world state or a subtree to a serde
/// format. The config (filters, entities, resource flag) is held independently
/// of the world, so a preconfigured saver, eg one that denies certain export
/// markers, can be reused, and the world is borrowed only at the moment it is
/// traversed ([`with_entity_tree`](Self::with_entity_tree)) or serialized
/// ([`save`](Self::save)).
///
/// The output is a [`SceneBody`]: every component a resolved value, the
/// save-game form.
///
/// # A dump is never a source
///
/// A dump and an authored source are distinct artifact roles and never share a
/// path. A `.bsx` entry is the authored *original* state of an application and
/// is never written here; what this produces is either a document the editor
/// owns (loaded through [`TemplateLoader`], re-saved through its retained
/// [`TemplateEntityMap`]) or a snapshot of a running world. A caller that writes
/// a dump over the file its scene was authored from has destroyed the original,
/// and no amount of key stability makes that recoverable.
#[derive(Default)]
pub struct TemplateSaver {
	component_filter: TemplateFilter,
	resource_filter: TemplateFilter,
	entities: Vec<Entity>,
	extract_resources: bool,
}

impl TemplateSaver {
	/// Creates an empty saver. Add entities with [`with_entity_tree`](Self::with_entity_tree)
	/// or [`with_entities`](Self::with_entities), then [`save`](Self::save).
	pub fn new() -> Self { Self::default() }

	/// Creates a saver that extracts all entities and resources.
	///
	/// Types marked [`Derived`](ReflectDerived) (the clocks, reactively
	/// recomputed paths) are skipped by the extractor itself, so this carries no
	/// deny list of its own.
	pub fn new_all(world: &World) -> Self {
		Self::new()
			.with_entities(world.iter_entities().map(|entity| entity.id()))
			.extract_resources()
	}

	/// Scopes serialization to an entity and its descendants.
	pub fn with_entity_tree(mut self, world: &World, entity: Entity) -> Self {
		self.collect_descendants(world, entity);
		self
	}

	/// Scopes serialization to a specific set of entities.
	pub fn with_entities(
		mut self,
		entities: impl IntoIterator<Item = Entity>,
	) -> Self {
		self.entities.extend(entities);
		self
	}

	/// Extracts all resources.
	pub fn extract_resources(mut self) -> Self {
		self.extract_resources = true;
		self
	}

	/// Allows a resource type to be serialized, narrowing an unset filter to
	/// the types allowed.
	pub fn allow_resource<T: Resource>(mut self) -> Self {
		self.resource_filter = self.resource_filter.allow::<T>();
		self
	}

	/// Denies a resource type from being serialized.
	pub fn deny_resource<T: Resource>(mut self) -> Self {
		self.resource_filter = self.resource_filter.deny::<T>();
		self
	}

	/// Denies a component type from being serialized.
	pub fn deny_component<T: Component>(mut self) -> Self {
		self.component_filter = self.component_filter.deny::<T>();
		self
	}

	/// Serialize `roots` and their descendants as one template, through the
	/// document's retained keys.
	///
	/// A re-save is a rewrite of the same entities rather than a fresh file that
	/// happens to look similar. One file is one keyspace however many roots it
	/// holds, so the [`TemplateEntityMap`] lives on the first root, which is also
	/// where a load lands it; a first save mints the keys and retains them there,
	/// so the next save is a rewrite like any other. A document anchored elsewhere
	/// (a scene, whose host holds the map) saves through
	/// [`save_roots_mapped`](Self::save_roots_mapped).
	pub fn save_roots(
		self,
		world: &mut World,
		media_type: MediaType,
		roots: impl IntoIterator<Item = Entity>,
	) -> Result<MediaBytes> {
		let roots = roots.into_iter().collect::<Vec<_>>();
		let entity_map = roots
			.first()
			.and_then(|root| {
				world.entity(*root).get::<TemplateEntityMap>().cloned()
			})
			.unwrap_or_default();
		let (bytes, entity_map) =
			self.save_roots_mapped(world, media_type, &roots, entity_map)?;
		if let Some(root) = roots.first() {
			world.entity_mut(*root).insert(entity_map);
		}
		Ok(bytes)
	}

	/// Serialize `roots` and their descendants as one template, `entity_map`
	/// seeding the file keys, returning the map the save wrote for the caller to
	/// retain wherever its document lives.
	///
	/// See [`roots_body`](Self::roots_body) for what a root's own parent
	/// becomes.
	pub fn save_roots_mapped(
		self,
		world: &mut World,
		media_type: MediaType,
		roots: &[Entity],
		entity_map: TemplateEntityMap,
	) -> Result<(MediaBytes, TemplateEntityMap)> {
		let (body, entity_map) = self.roots_body(world, roots, entity_map)?;
		body.to_bytes(media_type).map(|bytes| (bytes, entity_map))
	}

	/// `roots` and their descendants as one [`SceneBody`], `entity_map`
	/// seeding the file keys (an empty one mints every key), returning the map
	/// the save wrote for the caller to retain.
	///
	/// A root may sit under a parent (eg a loaded template reparented under a
	/// server); that [`ChildOf`] is detached before extracting and restored
	/// after, so the body carries no reference to a parent it does not hold.
	pub fn roots_body(
		mut self,
		world: &mut World,
		roots: &[Entity],
		entity_map: TemplateEntityMap,
	) -> Result<(SceneBody, TemplateEntityMap)> {
		// detach each root from its parent, remembering them to re-attach once
		// serialized.
		let roots_with_parents = roots
			.iter()
			.filter_map(|root| {
				world
					.entity(*root)
					.get::<ChildOf>()
					.map(|child_of| (*root, child_of.parent()))
			})
			.collect::<Vec<_>>();
		roots_with_parents.iter().for_each(|(root, _)| {
			world.entity_mut(*root).remove::<ChildOf>();
		});

		for root in roots {
			self = self.with_entity_tree(world, *root);
		}
		let result = self.body_mapped(world, entity_map);

		roots_with_parents.into_iter().for_each(|(root, parent)| {
			world.entity_mut(root).insert(ChildOf(parent));
		});
		result
	}

	/// Like [`save_roots`](Self::save_roots) but collects the roots from a query
	/// filter, eg `save_roots_filtered::<With<BeetSceneRoot>>`.
	pub fn save_roots_filtered<D: QueryFilter>(
		self,
		world: &mut World,
		media_type: MediaType,
	) -> Result<MediaBytes> {
		let roots = world
			.query_filtered::<Entity, D>()
			.iter(world)
			.collect::<Vec<_>>();
		self.save_roots(world, media_type, roots)
	}

	/// Serializes to [`MediaBytes`] using the given format with default options.
	pub fn save(
		self,
		world: &World,
		media_type: MediaType,
	) -> Result<MediaBytes> {
		self.save_with_options(world, media_type, default())
	}

	/// Serializes to [`MediaBytes`] using the given format and [`SerializeOptions`].
	pub fn save_with_options(
		self,
		world: &World,
		media_type: MediaType,
		options: SerializeOptions,
	) -> Result<MediaBytes> {
		self.save_mapped(world, media_type, options, default())
			.map(|(bytes, _)| bytes)
	}

	/// Serialize with `entity_map` seeding the file keys, returning the map the
	/// save wrote so the caller can retain it.
	///
	/// The mapped form of [`save_with_options`](Self::save_with_options): an
	/// empty map mints every key, which is what a first save wants.
	fn save_mapped(
		self,
		world: &World,
		media_type: MediaType,
		options: SerializeOptions,
		entity_map: TemplateEntityMap,
	) -> Result<(MediaBytes, TemplateEntityMap)> {
		let (body, entity_map) = self.body_mapped(world, entity_map)?;
		body.to_bytes_with_options(media_type, options)
			.map(|bytes| (bytes, entity_map))
	}

	/// The configured entities and resources as a [`SceneBody`], `entity_map`
	/// seeding the file keys.
	fn body_mapped(
		self,
		world: &World,
		entity_map: TemplateEntityMap,
	) -> Result<(SceneBody, TemplateEntityMap)> {
		let registry = world.resource::<AppTypeRegistry>();
		let registry = registry.read();
		let mut builder = TemplateBuilder::from_world(world, &registry)
			.with_component_filter(self.component_filter)
			.with_resource_filter(self.resource_filter)
			.with_entity_map(entity_map)
			.extract_entities(self.entities.into_iter());
		if self.extract_resources {
			builder = builder.extract_resources();
		}
		builder.build_mapped()
	}

	/// Collects an entity, its attribute entities and all its descendants into
	/// the entity set, stopping at a [`Derived`] subtree, which is never content.
	fn collect_descendants(&mut self, world: &World, entity: Entity) {
		let entity_ref = world.entity(entity);
		if entity_ref.contains::<Derived>() {
			return;
		}
		self.entities.push(entity);
		// an element's attributes are its own entities, owned through
		// `AttributeOf` rather than `ChildOf`, and part of it all the same
		let owned = entity_ref
			.get::<Attributes>()
			.into_iter()
			.flat_map(|attributes| attributes.iter())
			.chain(
				entity_ref
					.get::<Children>()
					.into_iter()
					.flat_map(|children| children.iter()),
			)
			.collect::<Vec<_>>();
		for child in owned {
			self.collect_descendants(world, child);
		}
	}
}

#[cfg(all(test, feature = "json"))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The standing tripwire for the derived-state rule: the exact set of
	/// component type paths a document scene dumps.
	///
	/// A new type appearing in this snapshot is either genuinely authored
	/// content or a cache that leaked into the format. The snapshot makes that
	/// a review decision instead of an accident nobody notices until a reopened
	/// document has grown fields it never had.
	#[beet_core::test]
	fn dumped_component_types() {
		let mut world =
			<(ScenePlugin, DocumentPlugin, MinimalTypesPlugin)>::world();
		let root = world
			.spawn((Document::new(value!({ "count": 1 })), children![(
				Value::default(),
				FieldRef::new("count")
			)]))
			.flush();
		// settle the sync so the derived path is actually on the field entity
		world.update_local();
		let field = world.entity(root).get::<Children>().unwrap()[0];
		world
			.entity(field)
			.contains::<ResolvedFieldPath>()
			.xpect_true();

		let scene = TemplateSaver::new()
			.save_roots(&mut world, MediaType::Json, [root])
			.unwrap()
			.deserialize::<Value>()
			.unwrap();
		let mut dumped = scene
			.get(SceneEntities::ENTITIES)
			.unwrap()
			.as_map()
			.unwrap()
			.0
			.keys()
			.map(|key| EntityKey::parse(key).unwrap())
			.map(|key| SceneEntities::entity_value(&scene, key).unwrap())
			.flat_map(|entity| entity.as_map().unwrap().0.keys())
			.cloned()
			.collect::<Vec<_>>();
		dumped.sort();
		dumped.dedup();
		dumped.join("\n").xpect_snapshot();
	}

	/// An authored `Disabled` is content a save keeps, where `Held` is never
	/// written and a load holds the prefab again.
	#[cfg(feature = "json")]
	#[beet_core::test]
	fn disabled_saves_and_held_does_not() {
		let new_world = || <(ScenePlugin, MinimalTypesPlugin)>::world();
		let mut world = new_world();
		let root = world.spawn(Name::new("root")).id();
		world.spawn((Name::new("off"), Disabled, ChildOf(root)));
		let prefab = world.spawn((Prefab, ChildOf(root))).id();
		world.spawn((Name::new("held"), ChildOf(prefab)));
		world.flush();
		let bytes = TemplateSaver::new()
			.save_roots(&mut world, MediaType::Json, [root])
			.unwrap();
		bytes
			.as_utf8()
			.unwrap()
			.xref()
			.xpect_contains("Disabled")
			.xpect_contains("Prefab")
			.xnot()
			.xpect_contains("Held");
		let mut loaded = new_world();
		let root = TemplateLoader::new(&mut loaded).load(&bytes).unwrap()[0];
		loaded.flush();
		let children = loaded.entity(root).get::<Children>().unwrap().to_vec();
		loaded
			.entity(children[0])
			.contains::<Disabled>()
			.xpect_true();
		let held = loaded.entity(children[1]).get::<Children>().unwrap()[0];
		loaded.entity(held).contains::<Held>().xpect_true();
	}
}
