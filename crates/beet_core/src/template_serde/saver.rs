use super::TemplateBuilder;
use super::TemplateFilter;
use super::serde::DynamicTemplateSerializer;
use crate::prelude::*;
use bevy::ecs::query::QueryFilter;

/// Configures and runs a serialization of world state or a subtree to a serde
/// format. The config (filters, entities, resource flag) is held independently
/// of the world, so a preconfigured saver, eg one that denies certain export
/// markers, can be reused, and the world is borrowed only at the moment it is
/// traversed ([`with_entity_tree`](Self::with_entity_tree)) or serialized
/// ([`save`](Self::save)).
///
/// The output is a resolved-value [`DynamicTemplate`]: every component is a
/// concrete value, the save-game form.
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

	/// Serialize `roots` and their descendants as one template.
	///
	/// A root may sit under a parent (eg a loaded template reparented under a
	/// server); that [`ChildOf`] is detached before serializing and restored
	/// after, so the saved template carries no dangling parent reference (which
	/// would fail to build on load).
	pub fn save_roots(
		mut self,
		world: &mut World,
		media_type: MediaType,
		roots: impl IntoIterator<Item = Entity>,
	) -> Result<MediaBytes> {
		let roots = roots.into_iter().collect::<Vec<_>>();
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

		for root in &roots {
			self = self.with_entity_tree(world, *root);
		}
		// the document's retained keys, so a re-save is a rewrite of the same
		// nodes rather than a fresh file that happens to look similar. One file
		// is one keyspace however many roots it holds, so the map lives on the
		// first root, which is also where a load lands it.
		let entity_map = roots
			.first()
			.and_then(|root| {
				world.entity(*root).get::<TemplateEntityMap>().cloned()
			})
			.unwrap_or_default();
		let result = self.save_mapped(world, media_type, default(), entity_map);
		// a first save mints the document's keys, so it retains them too: the
		// next save is then a rewrite like any other.
		if let (Some(root), Ok((_, entity_map))) = (roots.first(), &result) {
			world.entity_mut(*root).insert(entity_map.clone());
		}

		roots_with_parents.into_iter().for_each(|(root, parent)| {
			world.entity_mut(root).insert(ChildOf(parent));
		});
		result.map(|(bytes, _)| bytes)
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
		let (template, entity_map) = builder.build_mapped();
		let serializer = DynamicTemplateSerializer::new(&template, &registry);
		MediaBytes::serialize_with_options(media_type, &serializer, options)
			.map(|bytes| (bytes, entity_map))
	}

	/// Collects an entity and all its descendants into the entity set.
	fn collect_descendants(&mut self, world: &World, entity: Entity) {
		self.entities.push(entity);
		if let Some(children) = world.entity(entity).get::<Children>() {
			let children = children.iter().collect::<Vec<_>>();
			for child in children {
				self.collect_descendants(world, child);
			}
		}
	}
}

#[cfg(all(test, feature = "json"))]
mod test {
	use crate::prelude::*;

	/// The standing tripwire for the derived-state rule: the exact set of
	/// component type paths a document scene dumps.
	///
	/// A new type appearing in this snapshot is either genuinely authored
	/// content or a cache that leaked into the format. The snapshot makes that
	/// a review decision instead of an accident nobody notices until a reopened
	/// document has grown fields it never had.
	#[crate::test]
	fn dumped_component_types() {
		let mut world = <(DocumentPlugin, MinimalTypesPlugin)>::world();
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

		let json = TemplateSaver::new()
			.save_roots(&mut world, MediaType::Json, [root])
			.unwrap()
			.as_utf8()
			.unwrap()
			.xmap(serde_json::from_str::<serde_json::Value>)
			.unwrap();
		let mut dumped = json["nodes"]
			.as_object()
			.unwrap()
			.values()
			.flat_map(|node| node["components"].as_object().unwrap().keys())
			.cloned()
			.collect::<Vec<_>>();
		dumped.sort();
		dumped.dedup();
		dumped.join("\n").xpect_snapshot();
	}
}
