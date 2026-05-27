use super::DynamicEntity;
use super::DynamicWorld;
use super::WorldFilter;
use super::reflect_utils::clone_reflect_value;
use crate::prelude::*;
use alloc::collections::BTreeMap;
use bevy::ecs::component::ComponentId;
use bevy::ecs::entity_disabling::DefaultQueryFilters;
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::reflect::ReflectResource;
use bevy::ecs::resource::IS_RESOURCE;
use bevy_reflect::PartialReflect;
use bevy_reflect::TypeRegistry;
use core::any::TypeId;

/// Builds a [`DynamicWorld`] from a [`World`] by extracting some entities and resources.
///
/// # Component Extraction
///
/// By default, all components registered with [`ReflectComponent`] type data in the provided
/// [`TypeRegistry`] are extracted. This can be changed by [specifying a filter](Self::with_component_filter)
/// or by explicitly [allowing](Self::allow_component)/[denying](Self::deny_component) certain components.
///
/// # Resource Extraction
///
/// By default, all resources registered with [`ReflectResource`] type data in the provided
/// [`TypeRegistry`] are extracted. This can be changed by [specifying a filter](Self::with_resource_filter)
/// or by explicitly [allowing](Self::allow_resource)/[denying](Self::deny_resource) certain resources.
///
/// # Entity Order
///
/// Extracted entities are always stored in ascending order based on their [index](Entity::index).
///
/// # Example
/// ```
/// use beet_core::prelude::*;
/// #[derive(Component, Reflect, Default)]
/// #[reflect(Component)]
/// struct ComponentA;
/// let mut world = World::default();
/// world.init_resource::<AppTypeRegistry>();
/// world
///     .resource_mut::<AppTypeRegistry>()
///     .write()
///     .register::<ComponentA>();
/// let entity = world.spawn(ComponentA).id();
/// let type_registry = world.resource::<AppTypeRegistry>().read();
/// let dynamic_world = DynamicWorldBuilder::from_world(&world, &type_registry)
///     .extract_entity(entity)
///     .build();
/// ```
pub struct DynamicWorldBuilder<'w> {
	/// The resources that have been extracted so far.
	extracted_resources: BTreeMap<ComponentId, Box<dyn PartialReflect>>,
	/// The entities that have been extracted so far.
	extracted_entities: BTreeMap<Entity, DynamicEntity>,
	/// The filter to determine which components to extract.
	component_filter: WorldFilter,
	/// The filter to determine which resources to extract.
	resource_filter: WorldFilter,
	/// The world from which to build the dynamic world.
	original_world: &'w World,
	/// The type registry to use for extracting items from the world.
	type_registry: &'w TypeRegistry,
}

impl<'w> DynamicWorldBuilder<'w> {
	/// Prepare a builder that extracts entities and their components from the given [`World`].
	///
	/// The `type_registry` provides type information for extracting components and resources
	/// through reflection, usually acquired via `world.resource::<AppTypeRegistry>().read()`.
	pub fn from_world(world: &'w World, type_registry: &'w TypeRegistry) -> Self {
		Self {
			extracted_resources: default(),
			extracted_entities: default(),
			component_filter: WorldFilter::default(),
			resource_filter: WorldFilter::default(),
			original_world: world,
			type_registry,
		}
	}

	/// Specify a custom component [`WorldFilter`] to be used with this builder.
	#[must_use]
	pub fn with_component_filter(mut self, filter: WorldFilter) -> Self {
		self.component_filter = filter;
		self
	}

	/// Specify a custom resource [`WorldFilter`] to be used with this builder.
	#[must_use]
	pub fn with_resource_filter(mut self, filter: WorldFilter) -> Self {
		self.resource_filter = filter;
		self
	}

	/// Updates the filter to allow all component and resource types.
	pub fn allow_all(mut self) -> Self {
		self.component_filter = WorldFilter::allow_all();
		self.resource_filter = WorldFilter::allow_all();
		self
	}

	/// Updates the filter to deny all component and resource types.
	pub fn deny_all(mut self) -> Self {
		self.component_filter = WorldFilter::deny_all();
		self.resource_filter = WorldFilter::deny_all();
		self
	}

	/// Allows the given component type, `T`, to be included in the dynamic world.
	///
	/// The inverse of [`deny_component`](Self::deny_component).
	#[must_use]
	pub fn allow_component<T: Component>(mut self) -> Self {
		self.component_filter = self.component_filter.allow::<T>();
		self
	}

	/// Denies the given component type, `T`, from being included in the dynamic world.
	///
	/// The inverse of [`allow_component`](Self::allow_component).
	#[must_use]
	pub fn deny_component<T: Component>(mut self) -> Self {
		self.component_filter = self.component_filter.deny::<T>();
		self
	}

	/// Updates the filter to allow all component types.
	#[must_use]
	pub fn allow_all_components(mut self) -> Self {
		self.component_filter = WorldFilter::allow_all();
		self
	}

	/// Updates the filter to deny all component types.
	#[must_use]
	pub fn deny_all_components(mut self) -> Self {
		self.component_filter = WorldFilter::deny_all();
		self
	}

	/// Allows the given resource type, `T`, to be included in the dynamic world.
	///
	/// The inverse of [`deny_resource`](Self::deny_resource).
	#[must_use]
	pub fn allow_resource<T: Resource>(mut self) -> Self {
		self.resource_filter = self.resource_filter.allow::<T>();
		self
	}

	/// Denies the given resource type, `T`, from being included in the dynamic world.
	///
	/// The inverse of [`allow_resource`](Self::allow_resource).
	#[must_use]
	pub fn deny_resource<T: Resource>(mut self) -> Self {
		self.resource_filter = self.resource_filter.deny::<T>();
		self
	}

	/// Updates the filter to allow all resource types.
	#[must_use]
	pub fn allow_all_resources(mut self) -> Self {
		self.resource_filter = WorldFilter::allow_all();
		self
	}

	/// Updates the filter to deny all resource types.
	#[must_use]
	pub fn deny_all_resources(mut self) -> Self {
		self.resource_filter = WorldFilter::deny_all();
		self
	}

	/// Consume the builder, producing a [`DynamicWorld`].
	///
	/// To avoid entities without any components, call [`Self::remove_empty_entities`] first.
	#[must_use]
	pub fn build(self) -> DynamicWorld {
		DynamicWorld {
			resources: self.extracted_resources.into_values().collect(),
			entities: self.extracted_entities.into_values().collect(),
		}
	}

	/// Extract one entity from the builder's [`World`].
	///
	/// Re-extracting an entity that was already extracted has no effect.
	#[must_use]
	pub fn extract_entity(self, entity: Entity) -> Self {
		self.extract_entities(core::iter::once(entity))
	}

	/// Despawns all entities with no components.
	///
	/// These were likely created because none of their components were present in the
	/// provided type registry upon extraction.
	#[must_use]
	pub fn remove_empty_entities(mut self) -> Self {
		self.extracted_entities
			.retain(|_, entity| !entity.components.is_empty());
		self
	}

	/// Extract entities from the builder's [`World`].
	///
	/// Re-extracting an entity that was already extracted has no effect. To control which
	/// components are extracted, use the [`allow_component`](Self::allow_component) or
	/// [`deny_component`](Self::deny_component) helpers.
	#[must_use]
	pub fn extract_entities(
		mut self,
		entities: impl Iterator<Item = Entity>,
	) -> Self {
		for entity in entities {
			if self.extracted_entities.contains_key(&entity) {
				continue;
			}

			let mut entry = DynamicEntity {
				entity,
				components: Vec::new(),
			};

			let original_entity = self.original_world.entity(entity);
			if original_entity.contains_id(IS_RESOURCE) {
				continue;
			}

			// for each component on the entity, run reflection extraction through the filter
			for &component_id in original_entity.archetype().components().iter() {
				let mut extract_and_push = || {
					let type_id = self
						.original_world
						.components()
						.get_info(component_id)?
						.type_id()?;

					if self.component_filter.is_denied_by_id(type_id) {
						// component is in the denylist or _not_ in the allowlist
						return None;
					}

					let type_registration = self.type_registry.get(type_id)?;

					let component = type_registration
						.data::<ReflectComponent>()?
						.reflect(original_entity)?;

					entry.components.push(clone_reflect_value(
						component.as_partial_reflect(),
						type_registration,
					));
					Some(())
				};
				extract_and_push();
			}
			self.extracted_entities.insert(entity, entry);
		}

		self
	}

	/// Extract resources from the builder's [`World`].
	///
	/// Re-extracting a resource that was already extracted has no effect. To control which
	/// resources are extracted, use the [`allow_resource`](Self::allow_resource) or
	/// [`deny_resource`](Self::deny_resource) helpers.
	#[must_use]
	pub fn extract_resources(mut self) -> Self {
		// never extract the DefaultQueryFilters resource
		let original_world_dqf_id = self
			.original_world
			.components()
			.get_valid_id(TypeId::of::<DefaultQueryFilters>());

		for (component_id, entity) in
			self.original_world.resource_entities().iter()
		{
			if Some(component_id) == original_world_dqf_id {
				continue;
			}
			let mut extract_and_push = || {
				let type_id = self
					.original_world
					.components()
					.get_info(component_id)?
					.type_id()?;

				if self.resource_filter.is_denied_by_id(type_id) {
					// resource is in the denylist or _not_ in the allowlist
					return None;
				}

				let type_registration = self.type_registry.get(type_id)?;

				type_registration.data::<ReflectResource>()?;
				let component = type_registration
					.data::<ReflectComponent>()?
					.reflect(self.original_world.entity(entity))?;

				self.extracted_resources.insert(
					component_id,
					clone_reflect_value(
						component.as_partial_reflect(),
						type_registration,
					),
				);
				Some(())
			};
			extract_and_push();
		}

		self
	}
}

#[cfg(test)]
mod test {
	use super::DynamicWorldBuilder;
	use crate::prelude::*;
	use bevy::ecs::query::With;
	use bevy_reflect::TypeRegistry;

	#[derive(Component, Reflect, Default, Eq, PartialEq, Debug)]
	#[reflect(Component)]
	struct ComponentA;

	#[derive(Component, Reflect, Default, Eq, PartialEq, Debug)]
	#[reflect(Component)]
	struct ComponentB;

	#[derive(Resource, Reflect, Default, Eq, PartialEq, Debug)]
	#[reflect(Resource)]
	struct ResourceA;

	#[derive(Resource, Reflect, Default, Eq, PartialEq, Debug)]
	#[reflect(Resource)]
	struct ResourceB;

	#[crate::test]
	fn extract_one_entity() {
		let mut world = World::default();
		let mut type_registry = TypeRegistry::default();
		type_registry.register::<ComponentA>();
		let entity = world.spawn((ComponentA, ComponentB)).id();

		let dynamic_world = DynamicWorldBuilder::from_world(&world, &type_registry)
			.extract_entity(entity)
			.build();

		dynamic_world.entities.len().xpect_eq(1);
		dynamic_world.entities[0].entity.xpect_eq(entity);
		dynamic_world.entities[0].components.len().xpect_eq(1);
		dynamic_world.entities[0].components[0]
			.represents::<ComponentA>()
			.xpect_true();
	}

	#[crate::test]
	fn extract_one_entity_two_components() {
		let mut world = World::default();
		let mut type_registry = TypeRegistry::default();
		type_registry.register::<ComponentA>();
		type_registry.register::<ComponentB>();
		let entity = world.spawn((ComponentA, ComponentB)).id();

		let dynamic_world = DynamicWorldBuilder::from_world(&world, &type_registry)
			.extract_entity(entity)
			.build();

		dynamic_world.entities.len().xpect_eq(1);
		dynamic_world.entities[0].components.len().xpect_eq(2);
	}

	#[crate::test]
	fn extract_entity_order() {
		let mut world = World::default();
		let entity_a = world.spawn_empty().id();
		let entity_b = world.spawn_empty().id();
		let entity_c = world.spawn_empty().id();
		let entity_d = world.spawn_empty().id();

		let type_registry = TypeRegistry::default();
		let mut entities = DynamicWorldBuilder::from_world(&world, &type_registry)
			.extract_entity(entity_b)
			.extract_entities([entity_d, entity_a].into_iter())
			.extract_entity(entity_c)
			.build()
			.entities
			.into_iter();

		entities.next().map(|entry| entry.entity).xpect_eq(Some(entity_d));
		entities.next().map(|entry| entry.entity).xpect_eq(Some(entity_c));
		entities.next().map(|entry| entry.entity).xpect_eq(Some(entity_b));
		entities.next().map(|entry| entry.entity).xpect_eq(Some(entity_a));
	}

	#[crate::test]
	fn extract_query() {
		let mut world = World::default();
		let mut type_registry = TypeRegistry::default();
		type_registry.register::<ComponentA>();
		type_registry.register::<ComponentB>();

		let entity_a_b = world.spawn((ComponentA, ComponentB)).id();
		let entity_a = world.spawn(ComponentA).id();
		world.spawn(ComponentB);

		let mut query = world.query_filtered::<Entity, With<ComponentA>>();
		let dynamic_world = DynamicWorldBuilder::from_world(&world, &type_registry)
			.extract_entities(query.iter(&world))
			.build();

		dynamic_world.entities.len().xpect_eq(2);
		let mut entities = vec![
			dynamic_world.entities[0].entity,
			dynamic_world.entities[1].entity,
		];
		entities.sort();
		entities.xpect_eq(vec![entity_a, entity_a_b]);
	}

	#[crate::test]
	fn remove_componentless_entity() {
		let mut world = World::default();
		let mut type_registry = TypeRegistry::default();
		type_registry.register::<ComponentA>();

		let entity_a = world.spawn(ComponentA).id();
		let entity_b = world.spawn(ComponentB).id();

		let dynamic_world = DynamicWorldBuilder::from_world(&world, &type_registry)
			.extract_entities([entity_a, entity_b].into_iter())
			.remove_empty_entities()
			.build();

		dynamic_world.entities.len().xpect_eq(1);
		dynamic_world.entities[0].entity.xpect_eq(entity_a);
	}

	#[crate::test]
	fn extract_one_resource() {
		let mut world = World::default();
		let mut type_registry = TypeRegistry::default();
		type_registry.register::<ResourceA>();
		world.insert_resource(ResourceA);

		let dynamic_world = DynamicWorldBuilder::from_world(&world, &type_registry)
			.extract_resources()
			.build();

		dynamic_world.resources.len().xpect_eq(1);
		dynamic_world.resources[0].represents::<ResourceA>().xpect_true();
	}

	#[crate::test]
	fn extracts_allowed_components() {
		let mut world = World::default();
		let mut type_registry = TypeRegistry::default();
		type_registry.register::<ComponentA>();
		type_registry.register::<ComponentB>();

		let entity_a_b = world.spawn((ComponentA, ComponentB)).id();
		let entity_a = world.spawn(ComponentA).id();
		let entity_b = world.spawn(ComponentB).id();

		let dynamic_world = DynamicWorldBuilder::from_world(&world, &type_registry)
			.allow_component::<ComponentA>()
			.extract_entities([entity_a_b, entity_a, entity_b].into_iter())
			.build();

		dynamic_world.entities.len().xpect_eq(3);
		dynamic_world.entities[2].components[0]
			.represents::<ComponentA>()
			.xpect_true();
		dynamic_world.entities[1].components[0]
			.represents::<ComponentA>()
			.xpect_true();
		dynamic_world.entities[0].components.len().xpect_eq(0);
	}

	#[crate::test]
	fn does_not_extract_denied_components() {
		let mut world = World::default();
		let mut type_registry = TypeRegistry::default();
		type_registry.register::<ComponentA>();
		type_registry.register::<ComponentB>();

		let entity_a_b = world.spawn((ComponentA, ComponentB)).id();
		let entity_a = world.spawn(ComponentA).id();
		let entity_b = world.spawn(ComponentB).id();

		let dynamic_world = DynamicWorldBuilder::from_world(&world, &type_registry)
			.deny_component::<ComponentA>()
			.extract_entities([entity_a_b, entity_a, entity_b].into_iter())
			.build();

		dynamic_world.entities.len().xpect_eq(3);
		dynamic_world.entities[0].components[0]
			.represents::<ComponentB>()
			.xpect_true();
		dynamic_world.entities[1].components.len().xpect_eq(0);
		dynamic_world.entities[2].components[0]
			.represents::<ComponentB>()
			.xpect_true();
	}

	#[crate::test]
	fn does_not_extract_denied_resources() {
		let mut world = World::default();
		let mut type_registry = TypeRegistry::default();
		type_registry.register::<ResourceA>();
		type_registry.register::<ResourceB>();
		world.insert_resource(ResourceA);
		world.insert_resource(ResourceB);

		let dynamic_world = DynamicWorldBuilder::from_world(&world, &type_registry)
			.deny_resource::<ResourceA>()
			.extract_resources()
			.build();

		dynamic_world.resources.len().xpect_eq(1);
		dynamic_world.resources[0].represents::<ResourceB>().xpect_true();
	}

	#[crate::test]
	fn uses_from_reflect() {
		#[derive(Component, Reflect)]
		#[reflect(Component)]
		struct SomeType(i32);

		#[derive(Resource, Reflect)]
		#[reflect(Resource)]
		struct SomeResource(i32);

		let mut world = World::default();
		let mut type_registry = TypeRegistry::default();
		type_registry.register::<SomeType>();
		type_registry.register::<SomeResource>();

		world.insert_resource(SomeResource(123));
		let entity = world.spawn(SomeType(123)).id();

		let dynamic_world = DynamicWorldBuilder::from_world(&world, &type_registry)
			.extract_resources()
			.extract_entities(vec![entity].into_iter())
			.build();

		dynamic_world.entities[0].components[0]
			.try_as_reflect()
			.unwrap()
			.is::<SomeType>()
			.xpect_true();
		dynamic_world.resources[0]
			.try_as_reflect()
			.unwrap()
			.is::<SomeResource>()
			.xpect_true();
	}
}
