use bevy::ecs::entity::MapEntities;
use bevy::ecs::reflect::ReflectMapEntities;
use bevy::prelude::*;


/// This component will be replaced with a [`TargetAgent`] that points to the root [`Parent`] of this entity.
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub struct RootIsTargetAgent;

/// Attach this to behavior entities that require a target agent.
#[derive(Debug, PartialEq, Deref, DerefMut, Component, Reflect)]
#[reflect(Component, MapEntities, PartialEq)]
pub struct TargetAgent(pub Entity);

impl MapEntities for TargetAgent {
	fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
		**self = entity_mapper.map_entity(**self);
	}
}

pub fn set_root_as_target_agent(
	mut commands: Commands,
	parents: Query<&Parent>,
	query: Query<(Entity, &Parent), With<RootIsTargetAgent>>,
) {
	for (entity, parent) in query.iter() {
		let root = ParentExt::get_root(parent, &parents);
		commands
			.entity(entity)
			.remove::<RootIsTargetAgent>()
			.insert(TargetAgent(root));
	}
}

pub struct ParentExt;

impl ParentExt {
	pub fn visit(
		entity: Entity,
		parents: &Query<&Parent>,
		mut func: impl FnMut(Entity),
	) {
		func(entity);
		if let Ok(parent) = parents.get(entity) {
			Self::visit(**parent, parents, func);
		}
	}

	pub fn find<T>(
		entity: Entity,
		parents: &Query<&Parent>,
		mut func: impl FnMut(Entity) -> Option<T>,
	) -> Option<T> {
		if let Some(val) = func(entity) {
			return Some(val);
		}
		if let Ok(parent) = parents.get(entity) {
			Self::find(**parent, parents, func)
		} else {
			None
		}
	}


	pub fn get_root(parent: &Parent, parent_query: &Query<&Parent>) -> Entity {
		if let Ok(grandparent) = parent_query.get(**parent) {
			Self::get_root(grandparent, parent_query)
		} else {
			**parent
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let mut app = App::new();
		app.add_systems(Update, set_root_as_target_agent);

		let world = app.world_mut();
		let grandparent = world.spawn(RootIsTargetAgent).id();
		let parent =
			world.spawn(RootIsTargetAgent).set_parent(grandparent).id();
		let child = world.spawn(RootIsTargetAgent).set_parent(parent).id();


		app.update();

		expect(&app)
			.not()
			.to_have_component::<TargetAgent>(grandparent)?;
		expect(&app)
			.component(parent)?
			.to_be(&TargetAgent(grandparent))?;
		expect(&app)
			.component(child)?
			.to_be(&TargetAgent(grandparent))?;

		Ok(())
	}
}
