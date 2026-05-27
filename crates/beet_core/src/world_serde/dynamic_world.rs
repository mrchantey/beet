use super::DynamicWorldBuilder;
use crate::prelude::*;
use bevy::ecs::component::ComponentCloneBehavior;
use bevy::ecs::entity::EntityHashMap;
use bevy::ecs::entity::SceneEntityMapper;
use bevy::ecs::reflect::AppTypeRegistry;
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::reflect::ReflectResource;
use bevy::ecs::relationship::RelationshipHookMode;
use bevy_reflect::PartialReflect;
use bevy_reflect::TypeRegistry;

/// A collection of serializable resources and dynamic entities.
///
/// Each dynamic entity contains its own run-time defined set of components.
/// Build one with a [`DynamicWorldBuilder`] and write it back into a [`World`]
/// with [`write_to_world`](Self::write_to_world).
#[derive(Default)]
pub struct DynamicWorld {
	/// Resources stored in the dynamic world.
	pub resources: Vec<Box<dyn PartialReflect>>,
	/// Entities contained in the dynamic world.
	pub entities: Vec<DynamicEntity>,
}

/// A reflection-powered serializable representation of an entity and its components.
pub struct DynamicEntity {
	/// The identifier of the entity, unique within a [`DynamicWorld`].
	///
	/// Components that reference this entity must consistently use this identifier.
	pub entity: Entity,
	/// The boxed components belonging to this entity.
	pub components: Vec<Box<dyn PartialReflect>>,
}

impl DynamicWorld {
	/// Create a new dynamic world from a given world.
	///
	/// Panics if `world` does not contain [`AppTypeRegistry`]. Use [`Self::from_world_with`]
	/// to handle this case.
	pub fn from_world(world: &World) -> Self {
		let type_registry = world.resource::<AppTypeRegistry>().read();
		Self::from_world_with(world, &type_registry)
	}

	/// Create a new dynamic world from a given world and [`TypeRegistry`].
	///
	/// Extracts all entities and resources registered with the appropriate reflect type data.
	pub fn from_world_with(world: &World, type_registry: &TypeRegistry) -> Self {
		DynamicWorldBuilder::from_world(world, type_registry)
			.extract_entities(
				// sidestep default query filters by iterating archetypes directly,
				// so custom disabled components are still extracted
				world
					.archetypes()
					.iter()
					.flat_map(bevy::ecs::archetype::Archetype::entities)
					.map(bevy::ecs::archetype::ArchetypeEntity::id),
			)
			.extract_resources()
			.build()
	}

	/// Write the resources, dynamic entities, and their components into the given world.
	///
	/// Errors if a type is not registered in `type_registry` or doesn't reflect
	/// [`Component`](bevy::ecs::component::Component) or [`Resource`](bevy::ecs::resource::Resource).
	pub fn write_to_world_with(
		&self,
		world: &mut World,
		entity_map: &mut EntityHashMap<Entity>,
		type_registry: &TypeRegistry,
	) -> Result<()> {
		// ensure every dynamic entity has a corresponding world entity in the map
		for dynamic_entity in &self.entities {
			entity_map
				.entry(dynamic_entity.entity)
				.or_insert_with(|| world.spawn_empty().id());
		}

		for dynamic_entity in &self.entities {
			let entity = *entity_map
				.get(&dynamic_entity.entity)
				.expect("should have previously spawned an empty entity");

			// apply each component to the mapped entity
			for component in &dynamic_entity.components {
				let reflect_component =
					reflect_component(type_registry, component.as_ref())?;

				{
					let component_id = reflect_component.register_component(world);
					// SAFETY: we registered the component above, so the info exists
					#[expect(unsafe_code, reason = "this is faster")]
					let component_info = unsafe {
						world.components().get_info_unchecked(component_id)
					};
					if matches!(
						*component_info.clone_behavior(),
						ComponentCloneBehavior::Ignore
					) {
						continue;
					}
				}

				SceneEntityMapper::world_scope(entity_map, world, |world, mapper| {
					reflect_component.apply_or_insert_mapped(
						&mut world.entity_mut(entity),
						component.as_partial_reflect(),
						type_registry,
						mapper,
						RelationshipHookMode::Skip,
					);
				});
			}
		}

		// insert resources after all entities, so they are available for reference mapping
		for resource in &self.resources {
			let reflect_component =
				reflect_resource(type_registry, resource.as_ref())?;

			let resource_id = reflect_component.register_component(world);

			// override the existing resource value, or spawn one if absent
			let entity =
				if let Some(entity) = world.resource_entities().get(resource_id) {
					entity
				} else {
					world.spawn_empty().id()
				};

			SceneEntityMapper::world_scope(entity_map, world, |world, mapper| {
				reflect_component.apply_or_insert_mapped(
					&mut world.entity_mut(entity),
					resource.as_partial_reflect(),
					type_registry,
					mapper,
					RelationshipHookMode::Skip,
				);
			});
		}

		Ok(())
	}

	/// Write the resources, dynamic entities, and their components into the given world,
	/// using the world's [`AppTypeRegistry`].
	///
	/// See [`write_to_world_with`](Self::write_to_world_with).
	pub fn write_to_world(
		&self,
		world: &mut World,
		entity_map: &mut EntityHashMap<Entity>,
	) -> Result<()> {
		let registry = world.resource::<AppTypeRegistry>().clone();
		self.write_to_world_with(world, entity_map, &registry.read())
	}
}

/// Resolve the [`ReflectComponent`] for a reflected component value, erroring if its type is
/// missing a represented type, unregistered, or not reflecting [`Component`](bevy::ecs::component::Component).
fn reflect_component<'a>(
	type_registry: &'a TypeRegistry,
	value: &dyn PartialReflect,
) -> Result<&'a ReflectComponent> {
	let Some(type_info) = value.get_represented_type_info() else {
		bevybail!(
			"world contains dynamic type `{}` without a represented type, \
			consider setting it with `set_represented_type`",
			value.reflect_type_path()
		);
	};
	let Some(registration) = type_registry.get(type_info.type_id()) else {
		bevybail!(
			"world contains the reflected type `{}` but it was not found in the type registry, \
			consider registering it with `app.register_type::<T>()`",
			type_info.type_path()
		);
	};
	let Some(reflect_component) = registration.data::<ReflectComponent>() else {
		bevybail!(
			"world contains the unregistered component `{}`, \
			consider adding `#[reflect(Component)]` to your type",
			type_info.type_path()
		);
	};
	Ok(reflect_component)
}

/// Resolve the [`ReflectComponent`] backing a reflected resource value, erroring if its type is
/// missing a represented type, unregistered, or not reflecting [`Resource`](bevy::ecs::resource::Resource).
fn reflect_resource<'a>(
	type_registry: &'a TypeRegistry,
	value: &dyn PartialReflect,
) -> Result<&'a ReflectComponent> {
	let Some(type_info) = value.get_represented_type_info() else {
		bevybail!(
			"world contains dynamic type `{}` without a represented type, \
			consider setting it with `set_represented_type`",
			value.reflect_type_path()
		);
	};
	let Some(registration) = type_registry.get(type_info.type_id()) else {
		bevybail!(
			"world contains the reflected type `{}` but it was not found in the type registry, \
			consider registering it with `app.register_type::<T>()`",
			type_info.type_path()
		);
	};
	if registration.data::<ReflectResource>().is_none() {
		bevybail!(
			"world contains the unregistered resource `{}`, \
			consider adding `#[reflect(Resource)]` to your type",
			type_info.type_path()
		);
	}
	// ReflectResource existing implies ReflectComponent also exists
	registration
		.data::<ReflectComponent>()
		.expect("ReflectComponent is depended on by ReflectResource")
		.xok()
}

#[cfg(test)]
mod test {
	use super::DynamicWorld;
	use super::DynamicWorldBuilder;
	use crate::prelude::*;
	use bevy::ecs::entity::EntityHashMap;
	use bevy::ecs::entity::EntityMapper;

	#[derive(Resource, Reflect, MapEntities, Debug)]
	#[reflect(Resource, MapEntities)]
	struct TestResource {
		#[entities]
		entity_a: Entity,
		#[entities]
		entity_b: Entity,
	}

	#[crate::test]
	fn resource_entity_map_maps_entities() {
		let app_type_registry = AppTypeRegistry::default();
		app_type_registry.write().register::<TestResource>();

		let mut source_world = World::new();
		let original_entity_a = source_world.spawn_empty().id();
		let original_entity_b = source_world.spawn_empty().id();
		source_world.insert_resource(TestResource {
			entity_a: original_entity_a,
			entity_b: original_entity_b,
		});

		let dynamic_world = {
			let type_registry = app_type_registry.read();
			DynamicWorldBuilder::from_world(&source_world, &type_registry)
				.extract_resources()
				.extract_entity(original_entity_a)
				.extract_entity(original_entity_b)
				.build()
		};

		let mut entity_map = EntityHashMap::default();
		let mut destination_world = World::new();
		destination_world.insert_resource(app_type_registry);
		dynamic_world
			.write_to_world(&mut destination_world, &mut entity_map)
			.unwrap();

		let from_entity_a = *entity_map.get(&original_entity_a).unwrap();
		let from_entity_b = *entity_map.get(&original_entity_b).unwrap();
		let test_resource =
			destination_world.get_resource::<TestResource>().unwrap();
		test_resource.entity_a.xpect_eq(from_entity_a);
		test_resource.entity_b.xpect_eq(from_entity_b);
	}

	/// Reloading a dynamic world must only apply the entity map to components it
	/// actually defines, otherwise unrelated relationships get clobbered.
	#[crate::test]
	fn does_not_remap_components_outside_dynamic_world() {
		let mut world = World::new();
		world.init_resource::<AppTypeRegistry>();
		world
			.resource_mut::<AppTypeRegistry>()
			.write()
			.register::<ChildOf>();
		let original_parent = world.spawn_empty().id();
		let original_child = world.spawn_empty().id();
		world.entity_mut(original_parent).add_child(original_child);

		let dynamic_world = {
			let type_registry = world.resource::<AppTypeRegistry>().read();
			DynamicWorldBuilder::from_world(&world, &type_registry)
				.extract_entity(original_parent)
				.extract_entity(original_child)
				.build()
		};
		let mut entity_map = EntityHashMap::default();
		dynamic_world.write_to_world(&mut world, &mut entity_map).unwrap();

		let from_parent = *entity_map.get(&original_parent).unwrap();
		let from_child = *entity_map.get(&original_child).unwrap();

		// Original Parent <- Original Child <- Dynamic Parent <- Dynamic Child
		world.entity_mut(original_child).add_child(from_parent);
		// reloading must not touch the original child's parent
		dynamic_world.write_to_world(&mut world, &mut entity_map).unwrap();

		world
			.get_entity(original_child)
			.unwrap()
			.get::<ChildOf>()
			.unwrap()
			.parent()
			.xpect_eq(original_parent);
		world
			.get_entity(from_parent)
			.unwrap()
			.get::<ChildOf>()
			.unwrap()
			.parent()
			.xpect_eq(original_child);
		world
			.get_entity(from_child)
			.unwrap()
			.get::<ChildOf>()
			.unwrap()
			.parent()
			.xpect_eq(from_parent);
	}

	// Regression test for https://github.com/bevyengine/bevy/issues/14300
	#[crate::test]
	fn no_panic_in_map_entities_after_pending_entity_in_hook() {
		#[derive(Default, Component, Reflect)]
		#[reflect(Component)]
		struct A;

		#[derive(Component, Reflect)]
		#[reflect(Component)]
		struct B(pub Entity);

		impl MapEntities for B {
			fn map_entities<E: EntityMapper>(&mut self, entity_mapper: &mut E) {
				self.0 = entity_mapper.get_mapped(self.0);
			}
		}

		let registry = AppTypeRegistry::default();
		{
			let mut registry = registry.write();
			registry.register::<A>();
			registry.register::<B>();
		}

		let mut world = World::new();
		world.insert_resource(registry.clone());
		world.spawn((B(Entity::PLACEHOLDER), A));
		let dynamic_world = DynamicWorld::from_world(&world);

		let mut dst_world = World::new();
		dst_world.register_component_hooks::<A>().on_add(|mut world, _| {
			world.commands().spawn_empty();
		});
		dst_world.insert_resource(registry.clone());

		// should not panic on pending entities from the observer
		dynamic_world
			.write_to_world(&mut dst_world, &mut Default::default())
			.unwrap();
	}
}
