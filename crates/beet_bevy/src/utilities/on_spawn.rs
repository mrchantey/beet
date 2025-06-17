use beet_common_macros::ImplBundle;
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

/// A [`BundleEffect`] that runs a function when the entity is spawned.
#[derive(ImplBundle)]
pub struct OnSpawn<F: 'static + Send + Sync + FnOnce(&mut EntityWorldMut)>(
	pub F,
);

impl<F: Send + Sync + FnOnce(&mut EntityWorldMut)> OnSpawn<F> {
	/// Create a new [`OnSpawn`] effect.
	pub fn new(func: F) -> Self { Self(func) }
}

impl<F: 'static + Send + Sync + FnOnce(&mut EntityWorldMut)> BundleEffect
	for OnSpawn<F>
{
	fn apply(self, entity: &mut EntityWorldMut) { self.0(entity); }
}


/// A type erased [`BundleEffect`] that runs a function when the entity is spawned.
#[derive(ImplBundle)]
pub struct OnSpawnBoxed(
	pub Box<dyn 'static + Send + Sync + FnOnce(&mut EntityWorldMut)>,
);

impl OnSpawnBoxed {
	/// Create a new [`OnSpawnBoxed`] effect.
	pub fn new(
		func: impl 'static + Send + Sync + FnOnce(&mut EntityWorldMut),
	) -> Self {
		Self(Box::new(func))
	}
}

impl BundleEffect for OnSpawnBoxed {
	fn apply(self, entity: &mut EntityWorldMut) { (self.0)(entity); }
}
