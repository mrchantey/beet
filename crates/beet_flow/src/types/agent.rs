use beet_core::prelude::*;
use bevy::ecs::query::QueryData;
use bevy::ecs::query::QueryEntityError;
use bevy::ecs::query::QueryFilter;
use bevy::ecs::query::ROQueryItem;

/// Marker type to indicate this entity is the target of
#[derive(Debug, Copy, Clone, Reflect, Component)]
pub struct Agent;


#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = Actions)]
pub struct ActionOf(pub Entity);

#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = ActionOf, linked_spawn)]
pub struct Actions(Vec<Entity>);


/// A [`SystemParam`] used to get the agent for a particular action.
/// the agent is defined as either:
/// - The first [`Agent`] or [`ActionOf`] ancestor
/// - The root ancestor
#[derive(SystemParam)]
pub struct AgentQuery<'w, 's, D = (), F = ()>
where
	D: 'static + QueryData,
	F: 'static + QueryFilter,
{
	parents: Query<'w, 's, &'static ChildOf>,
	actions: Query<'w, 's, &'static ActionOf>,
	agents: Query<'w, 's, &'static Agent>,
	query: Query<'w, 's, D, F>,
}
impl<'w, 's, D, F> AgentQuery<'w, 's, D, F>
where
	D: 'static + QueryData,
	F: 'static + QueryFilter,
{
	/// Get the 'agent' entity for this action
	pub fn entity(&self, entity: Entity) -> Entity {
		// cache root to avoid double traversal
		let mut root = Entity::PLACEHOLDER;
		self.parents
			.iter_ancestors_inclusive(entity)
			.find_map(|entity| {
				root = entity;
				if self.agents.get(entity).is_ok() {
					Some(entity)
				} else if let Ok(action_of) = self.actions.get(entity) {
					Some(action_of.get())
				} else {
					None
				}
			})
			.unwrap_or(root)
	}

	pub fn get(
		&self,
		entity: Entity,
	) -> Result<ROQueryItem<'_, 's, D>, QueryEntityError> {
		let agent = self.entity(entity);
		self.query.get(agent)
	}

	pub fn get_mut(
		&mut self,
		entity: Entity,
	) -> Result<D::Item<'_, 's>, QueryEntityError> {
		let agent = self.entity(entity);
		self.query.get_mut(agent)
	}
}
