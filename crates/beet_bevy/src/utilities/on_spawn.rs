use bevy::ecs::bundle::BundleEffect;
use bevy::ecs::bundle::DynamicBundle;
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
pub struct OnSpawn<F>(pub F);

impl<F: Send + Sync + FnOnce(&mut EntityWorldMut)> OnSpawn<F> {
	/// Create a new [`OnSpawn`] effect.
	pub fn new(func: F) -> Self { Self(func) }
}

impl<F: Send + Sync + FnOnce(&mut EntityWorldMut)> BundleEffect for OnSpawn<F> {
	fn apply(self, entity: &mut EntityWorldMut) { self.0(entity); }
}


impl<T: 'static + Send + Sync + FnOnce(&mut EntityWorldMut)> DynamicBundle
	for OnSpawn<T>
{
	type Effect = Self;
	fn get_components(
		self,
		_func: &mut impl FnMut(
			bevy::ecs::component::StorageType,
			bevy::ptr::OwningPtr<'_>,
		),
	) -> Self::Effect {
		self
	}
}

unsafe impl<T: 'static + Send + Sync + FnOnce(&mut EntityWorldMut)> Bundle
	for OnSpawn<T>
{
	fn component_ids(
		_components: &mut bevy::ecs::component::ComponentsRegistrator,
		_ids: &mut impl FnMut(bevy::ecs::component::ComponentId),
	) {
	}

	fn get_component_ids(
		_components: &bevy::ecs::component::Components,
		_ids: &mut impl FnMut(Option<bevy::ecs::component::ComponentId>),
	) {
	}

	fn register_required_components(
		_components: &mut bevy::ecs::component::ComponentsRegistrator,
		_required_components: &mut bevy::ecs::component::RequiredComponents,
	) {
	}
}
