use bevy::ecs::bundle::BundleEffect;
use bevy::ecs::relationship::RelatedSpawner;
use bevy::ecs::spawn::SpawnRelatedBundle;
use bevy::ecs::spawn::SpawnWith;
use bevy::prelude::*;

/// Type helper for [`SpawnWith`]
pub fn spawn_with<T: RelationshipTarget, F>(
	func: F,
) -> SpawnRelatedBundle<T::Relationship, SpawnWith<F>>
where
	F: 'static + Send + Sync + FnOnce(&mut RelatedSpawner<T::Relationship>),
{
	T::spawn(SpawnWith(func))
}

/// Type helper for [`SpawnEffect`]
pub fn spawn_effect<F: Send + Sync + FnOnce(&mut EntityWorldMut)>(
	func: F,
) -> SpawnEffect<F> {
	SpawnEffect(func)
}

pub struct SpawnEffect<F>(pub F);

impl<F: Send + Sync + FnOnce(&mut EntityWorldMut)> BundleEffect
	for SpawnEffect<F>
{
	fn apply(self, entity: &mut EntityWorldMut) { self.0(entity); }
}
