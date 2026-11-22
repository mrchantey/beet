//! Target entity specification for action operations.
use crate::prelude::*;
use beet_core::prelude::*;

/// Specifies which entity an action should operate on.
///
/// Many actions need a target entity for their operations, such as inserting
/// or removing components. This enum provides a flexible way to specify that
/// target relative to the action entity.
///
/// # Example
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_action::prelude::*;
/// // Operate on the agent rather than the action entity itself
/// let target = TargetEntity::Agent;
/// # let _ = target;
/// ```
///
/// [`Other`](Self::Other) is an entity reference, so a clone or a scene load
/// remaps it to the entity it lands as.
#[derive(Debug, Default, Clone, Component, Reflect, PartialEq, Eq, Hash)]
#[reflect(Default, Component, MapEntities)]
#[component(map_entities)]
pub enum TargetEntity {
	/// Use the action entity itself (the caller).
	#[default]
	Action,
	/// Use the agent entity resolved via [`AgentQuery`].
	Agent,
	/// Specify an explicit entity to target.
	Other(Entity),
}

impl MapEntities for TargetEntity {
	fn map_entities<M: EntityMapper>(&mut self, mapper: &mut M) {
		if let Self::Other(entity) = self {
			*entity = mapper.get_mapped(*entity);
		}
	}
}

impl TargetEntity {
	/// Resolves the target entity for the given action.
	///
	/// Uses the provided [`AgentQuery`] to resolve the agent when
	/// [`TargetEntity::Agent`] is specified.
	pub fn get(&self, action: Entity, agent_query: &AgentQuery) -> Entity {
		match self {
			TargetEntity::Action => action,
			TargetEntity::Agent => agent_query.entity(action),
			TargetEntity::Other(entity) => *entity,
		}
	}

	/// Resolves the target entity asynchronously, using [`AgentQuery`] to
	/// resolve the agent when [`TargetEntity::Agent`] is specified.
	pub async fn get_async(
		&self,
		world: &AsyncWorld,
		action: Entity,
	) -> Entity {
		match self {
			TargetEntity::Action => action,
			TargetEntity::Agent => {
				AgentQuery::entity_async(world, action).await
			}
			TargetEntity::Other(entity) => *entity,
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A prefab whose actions name its own entities: every clone's references
	/// land on that clone's entities, never the prefab's or another clone's.
	#[beet_core::test]
	fn a_prefab_clone_remaps_its_targets() {
		let mut world = AsyncPlugin::world();
		let prefab = world.spawn(Prefab).id();
		let next = world.spawn(ChildOf(prefab)).id();
		world.spawn((
			TargetEntity::Other(next),
			RunNext::new(next),
			ChildOf(prefab),
		));
		let clones = [world.spawn_empty().id(), world.spawn_empty().id()].map(
			|target| {
				Prefab::clone_under(&mut world, prefab, target, ())
					.unwrap()
					.root
			},
		);
		for root in clones {
			let children =
				world.entity(root).get::<Children>().unwrap().to_vec();
			world
				.get::<TargetEntity>(children[1])
				.unwrap()
				.clone()
				.xpect_eq(TargetEntity::Other(children[0]));
			world
				.get::<RunNext>(children[1])
				.unwrap()
				.target
				.xpect_eq(Some(children[0]));
		}
	}
}
