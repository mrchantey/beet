use beet_core_macros::ImplBundle;
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



#[cfg(test)]
mod test {
	use std::sync::Arc;
	use std::sync::Mutex;

	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::expect;

	#[test]
	fn dfs() {
		let mut world = World::new();

		let numbers: Arc<Mutex<Vec<u32>>> = Arc::new(Mutex::new(Vec::new()));

		let numbers1 = numbers.clone();
		let numbers2 = numbers.clone();
		let numbers3 = numbers.clone();

		world.spawn((
			OnSpawn::new(move |entity_world_mut| {
				numbers1.lock().unwrap().push(1);
				entity_world_mut.insert(OnSpawn::new(move |_| {
					numbers2.lock().unwrap().push(2);
				}));
			}),
			OnSpawn::new(move |_| {
				numbers3.lock().unwrap().push(3);
			}),
		));

		expect(numbers.lock().unwrap().as_slice()).to_be([1, 2, 3].as_slice());
	}
}
