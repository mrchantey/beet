use super::DynamicWorldBuilder;
use super::WorldFilter;
use super::serde::DynamicWorldSerializer;
use crate::prelude::*;
use bevy::ecs::query::QueryFilter;

/// Configures and runs a serialization of world state or a subtree to various
/// formats. The config (filters, entities, resource flag) is held independently
/// of the world, so a preconfigured saver — eg one that denies certain export
/// markers — can be reused, and the world is borrowed only at the moment it is
/// traversed ([`with_entity_tree`](Self::with_entity_tree)) or serialized
/// ([`save`](Self::save)).
#[derive(Default)]
pub struct WorldSerdeSaver {
	component_filter: WorldFilter,
	resource_filter: WorldFilter,
	entities: Vec<Entity>,
	extract_resources: bool,
}

impl WorldSerdeSaver {
	/// Creates an empty saver. Add entities with [`with_entity_tree`](Self::with_entity_tree)
	/// or [`with_entities`](Self::with_entities), then [`save`](Self::save).
	pub fn new() -> Self { Self::default() }

	/// Creates a saver that extracts all entities and resources, denying [`Time<Real>`].
	pub fn new_default(world: &World) -> Self {
		Self::new()
			.with_entities(world.iter_entities().map(|entity| entity.id()))
			.deny_resource::<Time<Real>>()
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

	/// Serialize `roots` and their descendants as one scene.
	///
	/// A root may sit under a parent (eg a loaded scene reparented under a
	/// server); that [`ChildOf`] is detached before serializing and restored
	/// after, so the saved scene carries no dangling parent reference (which
	/// would fail to spawn on load).
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
		let result = self.save(world, media_type);

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
	pub fn save(self, world: &World, media_type: MediaType) -> Result<MediaBytes> {
		self.save_with_options(world, media_type, default())
	}

	/// Serializes to [`MediaBytes`] using the given format and [`SerializeOptions`].
	pub fn save_with_options(
		self,
		world: &World,
		media_type: MediaType,
		options: SerializeOptions,
	) -> Result<MediaBytes> {
		let registry = world.resource::<AppTypeRegistry>();
		let registry = registry.read();
		let mut builder = DynamicWorldBuilder::from_world(world, &registry)
			.with_component_filter(self.component_filter)
			.with_resource_filter(self.resource_filter)
			.extract_entities(self.entities.into_iter());
		if self.extract_resources {
			builder = builder.extract_resources();
		}
		let dyn_world = builder.build();
		let serializer = DynamicWorldSerializer::new(&dyn_world, &registry);
		MediaBytes::serialize_with_options(media_type, &serializer, options)
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
