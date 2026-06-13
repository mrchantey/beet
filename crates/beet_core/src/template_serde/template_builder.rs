//! Extracting a [`DynamicTemplate`] of resolved values from a [`World`].
//!
//! [`TemplateBuilder`] is the save side: it snapshots some entities and
//! resources by reflection into a [`DynamicTemplate`] whose component slots are
//! all [`ComponentSlot::Value`]. This is the save-game form, where every node is
//! already a resolved value. The deferred-template slot is produced by the
//! authoring front-ends (the parser and macros), not by extraction.

use crate::prelude::*;
use alloc::collections::BTreeMap;
use bevy::ecs::component::ComponentId;
use bevy::ecs::entity_disabling::DefaultQueryFilters;
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::reflect::ReflectResource;
use bevy::ecs::relationship::RelationshipAccessor;
use bevy::ecs::resource::IS_RESOURCE;
use bevy_reflect::PartialReflect;
use bevy_reflect::TypeRegistry;
use core::any::TypeId;

/// Builds a [`DynamicTemplate`] from a [`World`] by extracting some entities and
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
/// # Node Order
///
/// Extracted nodes are stored in extraction order, not entity-index order, so the
/// order a parent's children are walked into the template (eg by
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
/// let template = TemplateBuilder::from_world(&world, &type_registry)
///     .extract_entity(entity)
///     .build();
/// ```
pub struct TemplateBuilder<'w> {
	/// The resources that have been extracted so far.
	extracted_resources: BTreeMap<ComponentId, Box<dyn PartialReflect>>,
	/// The nodes that have been extracted so far, in extraction order.
	extracted_nodes: Vec<DynamicTemplateNode>,
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
			extracted_resources: default(),
			extracted_nodes: default(),
			extracted_entity_ids: default(),
			component_filter: TemplateFilter::default(),
			resource_filter: TemplateFilter::default(),
			original_world: world,
			type_registry,
		}
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

	/// Consume the builder, producing a [`DynamicTemplate`].
	///
	/// To avoid nodes without any components, call [`Self::remove_empty_nodes`]
	/// first.
	#[must_use]
	pub fn build(self) -> DynamicTemplate {
		DynamicTemplate {
			resources: self.extracted_resources.into_values().collect(),
			nodes: self.extracted_nodes,
		}
	}

	/// Extract one entity. Re-extracting an already-extracted entity is a no-op.
	#[must_use]
	pub fn extract_entity(self, entity: Entity) -> Self {
		self.extract_entities(core::iter::once(entity))
	}

	/// Drop nodes that have no components.
	///
	/// These were likely created because none of their components were present in
	/// the type registry upon extraction.
	#[must_use]
	pub fn remove_empty_nodes(mut self) -> Self {
		self.extracted_nodes
			.retain(|node| !node.components.is_empty());
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
		for entity in entities {
			if !self.extracted_entity_ids.insert(entity) {
				continue;
			}

			let mut node = DynamicTemplateNode {
				entity,
				components: Vec::new(),
			};

			let original_entity = self.original_world.entity(entity);
			if original_entity.contains_id(IS_RESOURCE) {
				continue;
			}

			// for each component on the entity, extract it through the filter.
			for &component_id in original_entity.archetype().components().iter() {
				let mut extract_and_push = || {
					let info = self
						.original_world
						.components()
						.get_info(component_id)?;

					// never extract a `RelationshipTarget` collection (eg `Children`,
					// `RenderRefOf`): it mirrors its `Relationship` source and is
					// rebuilt by the relationship hook on the build path. Serializing
					// it would double-apply the relation (the direct write plus the
					// hook), corrupting order and producing duplicate entries.
					if let Some(RelationshipAccessor::RelationshipTarget { .. }) =
						info.relationship_accessor()
					{
						return None;
					}

					let type_id = info.type_id()?;

					if self.component_filter.is_denied_by_id(type_id) {
						return None;
					}

					let type_registration = self.type_registry.get(type_id)?;
					let component = type_registration
						.data::<ReflectComponent>()?
						.reflect(original_entity)?;

					node.components.push(ComponentSlot::Value(
						reflect_ext::clone_reflect_value(
							component.as_partial_reflect(),
							type_registration,
						),
					));
					Some(())
				};
				extract_and_push();
			}
			self.extracted_nodes.push(node);
		}

		self
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
			let mut extract_and_push = || {
				let type_id = self
					.original_world
					.components()
					.get_info(component_id)?
					.type_id()?;

				if self.resource_filter.is_denied_by_id(type_id) {
					return None;
				}

				let type_registration = self.type_registry.get(type_id)?;
				type_registration.data::<ReflectResource>()?;
				let component = type_registration
					.data::<ReflectComponent>()?
					.reflect(self.original_world.entity(entity))?;

				self.extracted_resources.insert(
					component_id,
					reflect_ext::clone_reflect_value(
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

	/// True if the slot is a value that represents `T`.
	fn slot_represents<T: bevy_reflect::Typed>(slot: &ComponentSlot) -> bool {
		match slot {
			ComponentSlot::Value(value) => value.represents::<T>(),
			ComponentSlot::Template(_) => false,
		}
	}

	#[crate::test]
	fn extract_one_entity() {
		let mut world = World::default();
		let mut type_registry = TypeRegistry::default();
		type_registry.register::<ComponentA>();
		let entity = world.spawn((ComponentA, ComponentB)).id();

		let template = TemplateBuilder::from_world(&world, &type_registry)
			.extract_entity(entity)
			.build();

		template.nodes.len().xpect_eq(1);
		template.nodes[0].entity.xpect_eq(entity);
		template.nodes[0].components.len().xpect_eq(1);
		slot_represents::<ComponentA>(&template.nodes[0].components[0])
			.xpect_true();
	}

	#[crate::test]
	fn extract_one_entity_two_components() {
		let mut world = World::default();
		let mut type_registry = TypeRegistry::default();
		type_registry.register::<ComponentA>();
		type_registry.register::<ComponentB>();
		let entity = world.spawn((ComponentA, ComponentB)).id();

		let template = TemplateBuilder::from_world(&world, &type_registry)
			.extract_entity(entity)
			.build();

		template.nodes.len().xpect_eq(1);
		template.nodes[0].components.len().xpect_eq(2);
	}

	/// Nodes are stored in extraction order, not entity-index order, the
	/// children-order contract.
	#[crate::test]
	fn extract_node_order() {
		let mut world = World::default();
		let entity_a = world.spawn_empty().id();
		let entity_b = world.spawn_empty().id();
		let entity_c = world.spawn_empty().id();
		let entity_d = world.spawn_empty().id();

		let type_registry = TypeRegistry::default();
		let nodes = TemplateBuilder::from_world(&world, &type_registry)
			.extract_entity(entity_b)
			.extract_entities([entity_d, entity_a].into_iter())
			.extract_entity(entity_c)
			.build()
			.nodes
			.into_iter()
			.map(|node| node.entity)
			.collect::<Vec<_>>();

		// extraction order: b, then d, a, then c.
		nodes.xpect_eq(vec![entity_b, entity_d, entity_a, entity_c]);
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
		let template = TemplateBuilder::from_world(&world, &type_registry)
			.extract_entities(query.iter(&world))
			.build();

		template.nodes.len().xpect_eq(2);
		let mut entities =
			vec![template.nodes[0].entity, template.nodes[1].entity];
		entities.sort();
		entities.xpect_eq(vec![entity_a, entity_a_b]);
	}

	#[crate::test]
	fn remove_empty_node() {
		let mut world = World::default();
		let mut type_registry = TypeRegistry::default();
		type_registry.register::<ComponentA>();

		let entity_a = world.spawn(ComponentA).id();
		let entity_b = world.spawn(ComponentB).id();

		let template = TemplateBuilder::from_world(&world, &type_registry)
			.extract_entities([entity_a, entity_b].into_iter())
			.remove_empty_nodes()
			.build();

		template.nodes.len().xpect_eq(1);
		template.nodes[0].entity.xpect_eq(entity_a);
	}

	#[crate::test]
	fn extract_one_resource() {
		let mut world = World::default();
		let mut type_registry = TypeRegistry::default();
		type_registry.register::<ResourceA>();
		world.insert_resource(ResourceA);

		let template = TemplateBuilder::from_world(&world, &type_registry)
			.extract_resources()
			.build();

		template.resources.len().xpect_eq(1);
		template.resources[0].represents::<ResourceA>().xpect_true();
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

		let template = TemplateBuilder::from_world(&world, &type_registry)
			.allow_component::<ComponentA>()
			.extract_entities([entity_a_b, entity_a, entity_b].into_iter())
			.build();

		// extraction order: a_b, a, b. b has no allowed component.
		template.nodes.len().xpect_eq(3);
		slot_represents::<ComponentA>(&template.nodes[0].components[0])
			.xpect_true();
		slot_represents::<ComponentA>(&template.nodes[1].components[0])
			.xpect_true();
		template.nodes[2].components.len().xpect_eq(0);
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

		let template = TemplateBuilder::from_world(&world, &type_registry)
			.deny_component::<ComponentA>()
			.extract_entities([entity_a_b, entity_a, entity_b].into_iter())
			.build();

		// extraction order: a_b, a, b. a has only the denied component.
		template.nodes.len().xpect_eq(3);
		slot_represents::<ComponentB>(&template.nodes[0].components[0])
			.xpect_true();
		template.nodes[1].components.len().xpect_eq(0);
		slot_represents::<ComponentB>(&template.nodes[2].components[0])
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

		let template = TemplateBuilder::from_world(&world, &type_registry)
			.deny_resource::<ResourceA>()
			.extract_resources()
			.build();

		template.resources.len().xpect_eq(1);
		template.resources[0].represents::<ResourceB>().xpect_true();
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

		let template = TemplateBuilder::from_world(&world, &type_registry)
			.extract_resources()
			.extract_entities(vec![entity].into_iter())
			.build();

		match &template.nodes[0].components[0] {
			ComponentSlot::Value(value) => {
				value.try_as_reflect().unwrap().is::<SomeType>().xpect_true();
			}
			ComponentSlot::Template(_) => panic!("expected a value slot"),
		}
		template.resources[0]
			.try_as_reflect()
			.unwrap()
			.is::<SomeResource>()
			.xpect_true();
	}
}
