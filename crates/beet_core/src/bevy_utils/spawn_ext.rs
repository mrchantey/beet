//! Relationship spawning helpers.

use crate::prelude::*;
use bevy::ecs::relationship::RelatedSpawner;
use bevy::ecs::relationship::RelationshipTarget;
use bevy::ecs::spawn::SpawnRelatedBundle;
use bevy::ecs::spawn::SpawnWith;

/// Type helper for [`SpawnWith`], useful for spawning any number of related entities
/// like children.
pub fn spawn_with<T: RelationshipTarget, F>(
	func: F,
) -> SpawnRelatedBundle<T::Relationship, SpawnWith<F>>
where
	F: 'static + Send + Sync + FnOnce(&mut RelatedSpawner<T::Relationship>),
{
	T::spawn(SpawnWith(func))
}
