use super::DynamicWorldBuilder;
use super::WorldFilter;
use super::serde::DynamicWorldSerializer;
use crate::prelude::*;

/// Serializes world state or a subtree to various formats.
///
/// Use [`WorldSerdeSaver::new`] for the full world, or [`WorldSerdeSaver::with_entity_tree`]
/// to serialize only an entity and its descendants.
///
/// Extraction is deferred until [`WorldSerdeSaver::save`], as the underlying
/// [`DynamicWorldBuilder`] borrows the [`AppTypeRegistry`] for its lifetime.
pub struct WorldSerdeSaver<'a> {
	world: &'a World,
	component_filter: WorldFilter,
	resource_filter: WorldFilter,
	entities: Vec<Entity>,
	extract_resources: bool,
}

impl<'a> WorldSerdeSaver<'a> {
	/// Creates a saver for the entire world.
	pub fn new(world: &'a mut World) -> Self {
		Self {
			world,
			component_filter: WorldFilter::default(),
			resource_filter: WorldFilter::default(),
			entities: Vec::new(),
			extract_resources: false,
		}
	}

	/// Creates a saver that extracts all entities and resources, denying [`Time<Real>`].
	pub fn new_default(world: &'a mut World) -> Self {
		let all_entities: Vec<Entity> =
			world.query::<Entity>().iter(world).collect();
		Self::new(world)
			.with_entities(all_entities)
			.deny_resource::<Time<Real>>()
			.extract_resources()
	}

	/// Scopes serialization to an entity and its descendants.
	pub fn with_entity_tree(mut self, entity: Entity) -> Self {
		let mut entities = Vec::new();
		self.collect_descendants(entity, &mut entities);
		self.entities.extend(entities);
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

	/// Serializes to [`MediaBytes`] using the given format with default options.
	pub fn save(self, media_type: MediaType) -> Result<MediaBytes> {
		self.save_with_options(media_type, default())
	}

	/// Serializes to [`MediaBytes`] using the given format and [`SerializeOptions`].
	pub fn save_with_options(
		self,
		media_type: MediaType,
		options: SerializeOptions,
	) -> Result<MediaBytes> {
		let registry = self.world.resource::<AppTypeRegistry>();
		let registry = registry.read();
		let mut builder =
			DynamicWorldBuilder::from_world(self.world, &registry)
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

	/// Collects an entity and all its descendants into a flat list.
	fn collect_descendants(&self, entity: Entity, entities: &mut Vec<Entity>) {
		entities.push(entity);
		if let Some(children) = self.world.entity(entity).get::<Children>() {
			for child in children.iter() {
				self.collect_descendants(child, entities);
			}
		}
	}
}
