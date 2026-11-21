//! Extracting a [`SceneBody`] from a [`World`].
//!
//! [`TemplateBuilder`] is the save side: it snapshots some entities and
//! resources by reflection into a [`SceneBody`], every component a resolved
//! value. This is the save-game form.

use super::scene_reflect::ReferenceSentinels;
use crate::prelude::*;
use bevy::ecs::component::ComponentId;
use bevy::ecs::entity_disabling::DefaultQueryFilters;
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::reflect::ReflectResource;
use bevy::ecs::relationship::RelationshipAccessor;
use bevy::ecs::resource::IS_RESOURCE;
use bevy_reflect::PartialReflect;
use bevy_reflect::TypeRegistration;
use bevy_reflect::TypeRegistry;
use bevy_reflect::serde::TypedReflectSerializer;
use core::any::TypeId;
use serde::Serialize;

/// Builds a [`SceneBody`] from a [`World`] by extracting some entities and
/// resources as resolved values.
///
/// # Component Extraction
///
/// By default, all components registered with [`ReflectComponent`] type data in
/// the provided [`TypeRegistry`] are extracted. Restrict this with a
/// [filter](Self::with_component_filter) or the
/// [allow](Self::allow_component)/[deny](Self::deny_component) helpers.
///
/// # Resource Extraction
///
/// By default, all resources registered with [`ReflectResource`] type data are
/// extracted. Restrict this with a [filter](Self::with_resource_filter) or the
/// [allow](Self::allow_resource)/[deny](Self::deny_resource) helpers.
///
/// # Derived State
///
/// A type marked [`Derived`](ReflectDerived), and an entity under a [`Derived`]
/// mark, is skipped everywhere, whatever the filters say: derived state is not
/// content. The filters are for scoping a particular dump, never for excluding
/// state that is derived by nature.
///
/// # Unregistered Components
///
/// A component the loader could not resolve is held on its entity in an
/// [`Unregistered`]. It is never written as itself: each held component goes
/// back as one of the entity's own keys, beside the resolved ones, so a round
/// trip through a binary lacking a plugin loses nothing.
///
/// # References
///
/// An `Entity`-typed field is an entity reference, written as its target's
/// [`EntityKey`], never its live bits.
///
/// # File Order
///
/// Extracted entities are stored in extraction order, not entity-index order, so the
/// order a parent's children are walked into the body (eg by
/// [`with_entity_tree`](super::TemplateSaver::with_entity_tree)) is the order the
/// build path applies their `ChildOf`, rebuilding `Children` in the same order.
/// This is the children-order contract: it must survive a round-trip.
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
/// let body = TemplateBuilder::from_world(&world, &type_registry)
///     .extract_entity(entity)
///     .build()
///     .unwrap();
/// body.entities.len().xpect_eq(1);
/// ```
pub struct TemplateBuilder<'w> {
	/// The resources and entities extracted so far, the entities in extraction
	/// order.
	body: SceneBody,
	/// The first component or resource that could not be written.
	error: Option<BevyError>,
	/// The set of already-extracted entities, for deduplication.
	extracted_entity_ids: HashSet<Entity>,
	/// The filter determining which components to extract.
	component_filter: TemplateFilter,
	/// The filter determining which resources to extract.
	resource_filter: TemplateFilter,
	/// The world the template is built from.
	original_world: &'w World,
	/// The type registry used to extract items through reflection.
	type_registry: &'w TypeRegistry,
	/// The file keys every entity and every entity reference is written under.
	entity_map: TemplateEntityMap,
	/// Each unregistered component left out, with the number of entities it
	/// was on, logged once per type by [`Self::build_mapped`].
	skipped: HashMap<ComponentId, usize>,
}

impl<'w> TemplateBuilder<'w> {
	/// Prepares a builder that extracts entities and components from `world`.
	///
	/// `type_registry` provides type information for extraction, usually acquired
	/// via `world.resource::<AppTypeRegistry>().read()`.
	pub fn from_world(
		world: &'w World,
		type_registry: &'w TypeRegistry,
	) -> Self {
		Self {
			body: default(),
			error: None,
			extracted_entity_ids: default(),
			component_filter: TemplateFilter::default(),
			resource_filter: TemplateFilter::default(),
			original_world: world,
			type_registry,
			entity_map: default(),
			skipped: default(),
		}
	}

	/// Seed the builder with a loaded document's retained [`TemplateEntityMap`],
	/// so every entity it loaded is written back under its original file key and
	/// only a genuinely new entity mints one.
	///
	/// Without it every entity mints a fresh key, which is what a first save wants
	/// and the only case where that is correct.
	#[must_use]
	pub fn with_entity_map(mut self, entity_map: TemplateEntityMap) -> Self {
		self.entity_map = entity_map;
		self
	}

	/// Specify a custom component [`TemplateFilter`].
	#[must_use]
	pub fn with_component_filter(mut self, filter: TemplateFilter) -> Self {
		self.component_filter = filter;
		self
	}

	/// Specify a custom resource [`TemplateFilter`].
	#[must_use]
	pub fn with_resource_filter(mut self, filter: TemplateFilter) -> Self {
		self.resource_filter = filter;
		self
	}

	/// Allow all component and resource types.
	pub fn allow_all(mut self) -> Self {
		self.component_filter = TemplateFilter::allow_all();
		self.resource_filter = TemplateFilter::allow_all();
		self
	}

	/// Deny all component and resource types.
	pub fn deny_all(mut self) -> Self {
		self.component_filter = TemplateFilter::deny_all();
		self.resource_filter = TemplateFilter::deny_all();
		self
	}

	/// Allow the component type `T`, the inverse of [`deny_component`](Self::deny_component).
	#[must_use]
	pub fn allow_component<T: Component>(mut self) -> Self {
		self.component_filter = self.component_filter.allow::<T>();
		self
	}

	/// Deny the component type `T`, the inverse of [`allow_component`](Self::allow_component).
	#[must_use]
	pub fn deny_component<T: Component>(mut self) -> Self {
		self.component_filter = self.component_filter.deny::<T>();
		self
	}

	/// Allow all component types.
	#[must_use]
	pub fn allow_all_components(mut self) -> Self {
		self.component_filter = TemplateFilter::allow_all();
		self
	}

	/// Deny all component types.
	#[must_use]
	pub fn deny_all_components(mut self) -> Self {
		self.component_filter = TemplateFilter::deny_all();
		self
	}

	/// Allow the resource type `T`, the inverse of [`deny_resource`](Self::deny_resource).
	#[must_use]
	pub fn allow_resource<T: Resource>(mut self) -> Self {
		self.resource_filter = self.resource_filter.allow::<T>();
		self
	}

	/// Deny the resource type `T`, the inverse of [`allow_resource`](Self::allow_resource).
	#[must_use]
	pub fn deny_resource<T: Resource>(mut self) -> Self {
		self.resource_filter = self.resource_filter.deny::<T>();
		self
	}

	/// Allow all resource types.
	#[must_use]
	pub fn allow_all_resources(mut self) -> Self {
		self.resource_filter = TemplateFilter::allow_all();
		self
	}

	/// Deny all resource types.
	#[must_use]
	pub fn deny_all_resources(mut self) -> Self {
		self.resource_filter = TemplateFilter::deny_all();
		self
	}

	/// Consume the builder, producing a [`SceneBody`], or the first error an
	/// extraction met: a component whose value a scene cannot hold.
	///
	/// To avoid entities without any components, call [`Self::remove_empty_entities`]
	/// first.
	pub fn build(self) -> Result<SceneBody> {
		self.build_mapped().map(|(body, _)| body)
	}

	/// Consume the builder, producing a [`SceneBody`] and the
	/// [`TemplateEntityMap`] its keys came from, for the caller to retain.
	pub fn build_mapped(self) -> Result<(SceneBody, TemplateEntityMap)> {
		self.log_skipped();
		match self.error {
			Some(error) => Err(error),
			None => Ok((self.body, self.entity_map)),
		}
	}

	/// Extract one entity. Re-extracting an already-extracted entity is a no-op.
	#[must_use]
	pub fn extract_entity(self, entity: Entity) -> Self {
		self.extract_entities(core::iter::once(entity))
	}

	/// Drop entities that have no components.
	///
	/// These were likely created because none of their components were present in
	/// the type registry upon extraction.
	#[must_use]
	pub fn remove_empty_entities(mut self) -> Self {
		self.body.entities.retain(|_, entity| !entity.is_empty());
		self
	}

	/// Extract entities. Re-extracting an already-extracted entity is a no-op.
	///
	/// Control which components are extracted with the
	/// [allow](Self::allow_component)/[deny](Self::deny_component) helpers.
	#[must_use]
	pub fn extract_entities(
		mut self,
		entities: impl Iterator<Item = Entity>,
	) -> Self {
		// a resource entity is not a serialized entity, so it must never take a
		// file key; nor is derived state, which is rebuilt from what is
		let world = self.original_world;
		let entities = entities
			.filter(|entity| !world.entity(*entity).contains_id(IS_RESOURCE))
			.filter(|entity| !Derived::contains(world, *entity))
			.collect::<Vec<_>>();
		// key every entity first, in extraction order, so an entity reference
		// resolves to the same file key whether it points back or forward.
		let entities = entities
			.into_iter()
			.filter(|entity| self.extracted_entity_ids.insert(*entity))
			.collect::<Vec<_>>();
		for entity in &entities {
			self.entity_map.file_key(*entity);
		}

		for entity in entities {
			let key = self.entity_map.file_key(entity);
			let extracted = self.extract_components(entity);
			match extracted {
				Ok(extracted) => {
					self.body.entities.insert(key, extracted);
				}
				Err(error) => {
					self.error.get_or_insert(error);
				}
			}
		}

		self
	}

	/// Every component of `entity` the filters and the registry let through,
	/// as data.
	fn extract_components(&mut self, entity: Entity) -> Result<EntityBody> {
		let world = self.original_world;
		let original_entity = world.entity(entity);
		let mut resolved = EntityBody::default();
		// what a loader held goes back as the entity's own keys, never as the
		// marker holding it, read directly so it needs no registration
		let held = original_entity
			.get::<Unregistered>()
			.map(|unregistered| (**unregistered).clone())
			.unwrap_or_default();

		for &component_id in original_entity.archetype().components().iter() {
			let Some(info) = world.components().get_info(component_id) else {
				continue;
			};
			// never extract a `RelationshipTarget` collection (eg `Children`,
			// `PortalOf`): it mirrors its `Relationship` source and is
			// rebuilt by the relationship hook on the build path. Serializing
			// it would double-apply the relation (the direct write plus the
			// hook), corrupting order and producing duplicate entries.
			if let Some(RelationshipAccessor::RelationshipTarget { .. }) =
				info.relationship_accessor()
			{
				continue;
			}
			let Some(type_id) = info.type_id() else {
				continue;
			};
			if type_id == TypeId::of::<Unregistered>()
				|| self.component_filter.is_denied_by_id(type_id)
			{
				continue;
			}
			// an unregistered component cannot be dumped, so it is absent
			// from the file, tallied for the one line `build` logs
			let Some(registration) = self.type_registry.get(type_id) else {
				*self.skipped.entry(component_id).or_default() += 1;
				continue;
			};
			// derived state is never authored content, so it never dumps
			if registration.data::<ReflectDerived>().is_some() {
				continue;
			}
			let Some(reflect_component) =
				registration.data::<ReflectComponent>()
			else {
				continue;
			};
			let Some(component) = reflect_component.reflect(original_entity)
			else {
				continue;
			};
			let value = match reflect_ext::clone_reflect_value(
				component.as_partial_reflect(),
				registration,
			) {
				Ok(value) => value,
				Err(err) => {
					warn!(
						"skipping uncloneable component `{}` on {entity}: {err}",
						info.name()
					);
					continue;
				}
			};
			let type_path = registration.type_info().type_path();
			let value = self
				.component_value(value, registration, reflect_component)
				.map_err(|err| {
					bevyhow!("component `{type_path}` on {entity}: {err}")
				})?;
			resolved.insert(type_path, value);
		}
		// a resolved component wins the key a held one also names
		for (name, value) in Map::from(held) {
			if !resolved.contains_key(&name) {
				resolved.insert(name, value);
			}
		}
		resolved.xok()
	}

	/// One `debug!` per unregistered component type left out, so a missing
	/// route on the next boot can be traced back without a line per entity.
	fn log_skipped(&self) {
		let components = self.original_world.components();
		let mut skipped = self
			.skipped
			.iter()
			.filter_map(|(id, count)| {
				components
					.get_info(*id)
					.map(|info| (info.name().to_string(), *count))
			})
			.collect::<Vec<_>>();
		skipped.sort();
		for (name, count) in skipped {
			debug!(
				"skipped unregistered component `{name}` on {count} entities"
			);
		}
	}

	/// A component as data, each entity it references written as that
	/// entity's file key.
	fn component_value(
		&mut self,
		mut component: Box<dyn PartialReflect>,
		registration: &TypeRegistration,
		reflect_component: &ReflectComponent,
	) -> Result<Value> {
		let mut references = ReferenceSentinels::default();
		if let Some(reflect) = component.try_as_reflect_mut() {
			reflect_component.map_entities(reflect, &mut references);
		}
		Self::reflect_value(
			component.as_ref(),
			registration,
			self.type_registry,
		)
		.and_then(|value| references.keyed(value, &mut self.entity_map))
	}

	/// A reflected value as data, through its registered serde form.
	fn reflect_value(
		value: &dyn PartialReflect,
		registration: &TypeRegistration,
		registry: &TypeRegistry,
	) -> Result<Value> {
		TypedReflectSerializer::new(value, registry)
			.serialize(ValueSerializer)
			.map_err(|err| {
				bevyhow!("`{}`: {err}", registration.type_info().type_path())
			})
	}

	/// Extract resources. Re-extracting an already-extracted resource is a no-op.
	///
	/// Control which resources are extracted with the
	/// [allow](Self::allow_resource)/[deny](Self::deny_resource) helpers.
	#[must_use]
	pub fn extract_resources(mut self) -> Self {
		// never extract the DefaultQueryFilters resource.
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
			let extract_and_push = || {
				let type_id = self
					.original_world
					.components()
					.get_info(component_id)?
					.type_id()?;

				if self.resource_filter.is_denied_by_id(type_id) {
					return None;
				}

				let type_registration = self.type_registry.get(type_id)?;
				if type_registration.data::<ReflectDerived>().is_some() {
					return None;
				}
				type_registration.data::<ReflectResource>()?;
				let component = type_registration
					.data::<ReflectComponent>()?
					.reflect(self.original_world.entity(entity))?;

				let value = reflect_ext::clone_reflect_value(
					component.as_partial_reflect(),
					type_registration,
				)
				.inspect_err(|err| {
					warn!(
						"skipping uncloneable resource `{}`: {err}",
						type_registration.type_info().type_path()
					)
				})
				.ok()?;
				Some((type_registration, value))
			};
			let Some((registration, value)) = extract_and_push() else {
				continue;
			};
			match Self::reflect_value(
				value.as_ref(),
				registration,
				self.type_registry,
			) {
				Ok(value) => {
					self.body
						.resources
						.insert(registration.type_info().type_path(), value);
				}
				Err(error) => {
					self.error.get_or_insert(error);
				}
			}
		}

		self
	}
}

#[cfg(test)]
mod test {
	use super::TemplateBuilder;
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

	/// The file keys a body holds, in file order.
	fn keys(body: &SceneBody) -> Vec<&str> {
		body.entities.keys().map(EntityKey::as_str).collect()
	}

	/// The entity a body holds at the minted key `index`.
	fn entity(body: &SceneBody, index: u32) -> &EntityBody {
		&body.entities[&EntityKey::from(index)]
	}

	#[crate::test]
	fn extract_one_entity() {
		let mut world = World::default();
		let mut type_registry = TypeRegistry::default();
		type_registry.register::<ComponentA>();
		let entity_id = world.spawn((ComponentA, ComponentB)).id();

		let body = TemplateBuilder::from_world(&world, &type_registry)
			.extract_entity(entity_id)
			.build()
			.unwrap();

		// keys are minted from zero in extraction order, never derived from the
		// live entity's bits
		keys(&body).xpect_eq(vec!["0"]);
		entity(&body, 0).len().xpect_eq(1);
		entity(&body, 0).contains_typed::<ComponentA>().xpect_true();
	}

	#[crate::test]
	fn extract_one_entity_two_components() {
		let mut world = World::default();
		let mut type_registry = TypeRegistry::default();
		type_registry.register::<ComponentA>();
		type_registry.register::<ComponentB>();
		let entity_id = world.spawn((ComponentA, ComponentB)).id();

		let body = TemplateBuilder::from_world(&world, &type_registry)
			.extract_entity(entity_id)
			.build()
			.unwrap();

		body.entities.len().xpect_eq(1);
		entity(&body, 0).len().xpect_eq(2);
	}

	/// Entities are stored in extraction order, not entity-index order, the
	/// children-order contract. A first save mints keys along that same order,
	/// so the file reads `0`, `1`, `2`, `3`.
	#[crate::test]
	fn extract_entity_order() {
		let mut world = World::default();
		let entity_a = world.spawn_empty().id();
		let entity_b = world.spawn_empty().id();
		let entity_c = world.spawn_empty().id();
		let entity_d = world.spawn_empty().id();

		let type_registry = TypeRegistry::default();
		let (body, entity_map) =
			TemplateBuilder::from_world(&world, &type_registry)
				.extract_entity(entity_b)
				.extract_entities([entity_d, entity_a].into_iter())
				.extract_entity(entity_c)
				.build_mapped()
				.unwrap();

		keys(&body).xpect_eq(vec!["0", "1", "2", "3"]);
		// extraction order: b, then d, a, then c.
		[entity_b, entity_d, entity_a, entity_c]
			.map(|entity| entity_map.get_file_key(entity).unwrap().to_string())
			.xpect_eq(["0", "1", "2", "3"].map(str::to_string));
	}

	#[crate::test]
	fn extract_query() {
		let mut world = World::default();
		let mut type_registry = TypeRegistry::default();
		type_registry.register::<ComponentA>();
		type_registry.register::<ComponentB>();

		world.spawn((ComponentA, ComponentB));
		world.spawn(ComponentA);
		world.spawn(ComponentB);

		let mut query = world.query_filtered::<Entity, With<ComponentA>>();
		let body = TemplateBuilder::from_world(&world, &type_registry)
			.extract_entities(query.iter(&world))
			.build()
			.unwrap();

		keys(&body).xpect_eq(vec!["0", "1"]);
	}

	#[crate::test]
	fn remove_empty_entity() {
		let mut world = World::default();
		let mut type_registry = TypeRegistry::default();
		type_registry.register::<ComponentA>();

		let entity_a = world.spawn(ComponentA).id();
		let entity_b = world.spawn(ComponentB).id();

		let body = TemplateBuilder::from_world(&world, &type_registry)
			.extract_entities([entity_a, entity_b].into_iter())
			.remove_empty_entities()
			.build()
			.unwrap();

		keys(&body).xpect_eq(vec!["0"]);
	}

	#[crate::test]
	fn extract_one_resource() {
		let mut world = World::default();
		let mut type_registry = TypeRegistry::default();
		type_registry.register::<ResourceA>();
		world.insert_resource(ResourceA);

		let body = TemplateBuilder::from_world(&world, &type_registry)
			.extract_resources()
			.build()
			.unwrap();

		body.resources.len().xpect_eq(1);
		body.resources.contains_typed::<ResourceA>().xpect_true();
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

		let body = TemplateBuilder::from_world(&world, &type_registry)
			.allow_component::<ComponentA>()
			.extract_entities([entity_a_b, entity_a, entity_b].into_iter())
			.build()
			.unwrap();

		// extraction order: a_b, a, b. b has no allowed component.
		body.entities.len().xpect_eq(3);
		entity(&body, 0).contains_typed::<ComponentA>().xpect_true();
		entity(&body, 0).len().xpect_eq(1);
		entity(&body, 1).contains_typed::<ComponentA>().xpect_true();
		entity(&body, 2).len().xpect_eq(0);
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

		let body = TemplateBuilder::from_world(&world, &type_registry)
			.deny_component::<ComponentA>()
			.extract_entities([entity_a_b, entity_a, entity_b].into_iter())
			.build()
			.unwrap();

		// extraction order: a_b, a, b. a has only the denied component.
		body.entities.len().xpect_eq(3);
		entity(&body, 0).contains_typed::<ComponentB>().xpect_true();
		entity(&body, 0).len().xpect_eq(1);
		entity(&body, 1).len().xpect_eq(0);
		entity(&body, 2).contains_typed::<ComponentB>().xpect_true();
	}

	#[crate::test]
	fn does_not_extract_denied_resources() {
		let mut world = World::default();
		let mut type_registry = TypeRegistry::default();
		type_registry.register::<ResourceA>();
		type_registry.register::<ResourceB>();
		world.insert_resource(ResourceA);
		world.insert_resource(ResourceB);

		let body = TemplateBuilder::from_world(&world, &type_registry)
			.deny_resource::<ResourceA>()
			.extract_resources()
			.build()
			.unwrap();

		body.resources.len().xpect_eq(1);
		body.resources.contains_typed::<ResourceB>().xpect_true();
	}

	/// A component is written through its reflected form, a plain value under
	/// its type path.
	#[crate::test]
	fn writes_values() {
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
		let entity_id = world.spawn(SomeType(123)).id();

		let body = TemplateBuilder::from_world(&world, &type_registry)
			.extract_resources()
			.extract_entities(vec![entity_id].into_iter())
			.build()
			.unwrap();

		entity(&body, 0)
			.get(SomeType::type_path())
			.unwrap()
			.clone()
			.xpect_eq(Value::Int(123));
		body.resources
			.get(SomeResource::type_path())
			.unwrap()
			.clone()
			.xpect_eq(Value::Int(123));
	}
}
