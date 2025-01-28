use crate::events::OnRun;
use bevy::ecs::component::StorageType;
use bevy::ecs::entity::MapEntities;
use bevy::ecs::reflect::ReflectMapEntities;
use bevy::prelude::*;

/// Very simple pre-entity relations mechanic,
/// add this as an outgoing relation to entities with actions and other components that require it.
#[derive(Debug, PartialEq, Deref, DerefMut, Component, Reflect)]
#[reflect(Component, MapEntities, PartialEq)]
pub struct TargetEntity(pub Entity);

impl MapEntities for TargetEntity {
	fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
		**self = entity_mapper.map_entity(**self);
	}
}

/// Adds a [`TargetEntity`] that points to the root [`Parent`] of this entity.
///
/// If you need to dynamically update [`TargetEntity`] whenever you reparent a tree, add the
/// [`DynamicRootIsTargetEntity`] component to the root of the tree.
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
#[require(TargetEntity(||TargetEntity(Entity::PLACEHOLDER)))]
pub struct RootIsTargetEntity;

pub fn set_root_as_target_entity(
	parents: Query<&Parent>,
	mut query: Query<(Entity, &mut TargetEntity), Added<RootIsTargetEntity>>,
) {
	for (entity, mut target_entity) in query.iter_mut() {
		let root = parents.root_ancestor(entity);
		target_entity.0 = root;
	}
}

/// If present, will monitor if this entity has its Parent changed, and if so, will repair the
/// TargetEntity of all descendants with a [`RootIsTargetEntity`] component.
#[derive(Debug, Reflect)]
#[reflect(Component)]
pub struct DynamicRootIsTargetEntity;

impl Component for DynamicRootIsTargetEntity {
	const STORAGE_TYPE: StorageType = StorageType::Table;
	fn register_component_hooks(
		hooks: &mut bevy::ecs::component::ComponentHooks,
	) {
		hooks.on_add(|mut world, entity, _component_id| {
			let mut observer = Observer::new(fix_target_on_run);
			observer.watch_entity(entity);
			world.commands().spawn(observer);
		});
	}
}

fn fix_target_on_run(
	t: Trigger<OnRun>,
	q_parent_changed: Query<
		(),
		(With<DynamicRootIsTargetEntity>, Changed<Parent>),
	>,
	mut q_children: Query<&mut TargetEntity, With<RootIsTargetEntity>>,
	children_query: Query<&Children>,
	parent_query: Query<&Parent>,
) {
	if !q_parent_changed.contains(t.entity()) {
		// This means that the parent hasn't changed since the last time OnRun was triggered
		// for this entity - so we avoid descending the hierarchy.
		//
		// Changed<> has the desired behavior because this system is specific to an entity,
		// not a global observer.
		return;
	}
	let entity = t.entity();
	let root = parent_query.root_ancestor(entity);
	for child in children_query.iter_descendants(entity) {
		if let Ok(mut target_entity) = q_children.get_mut(child) {
			target_entity.0 = root;
		}
	}
}
