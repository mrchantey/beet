use crate::bevybail;
use beet_core_macros::ImplBundle;
use bevy::ecs::bundle::BundleEffect;
use bevy::ecs::relationship::RelatedSpawner;
use bevy::ecs::relationship::Relationship;
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
#[derive(Clone, ImplBundle)]
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


#[derive(Component)]
pub struct OnSpawnDeferred(
	pub Box<dyn 'static + Send + Sync + FnOnce(&mut EntityWorldMut) -> Result>,
);

impl OnSpawnDeferred {
	/// Create a new [`OnSpawnDeferred`] effect.
	pub fn new(
		func: impl 'static + Send + Sync + FnOnce(&mut EntityWorldMut) -> Result,
	) -> Self {
		Self(Box::new(func))
	}


	/// Insert this bundle into the entity on spawn.
	pub fn insert(bundle: impl Bundle) -> Self {
		Self::new(move |entity| {
			entity.insert(bundle);
			Ok(())
		})
	}

	/// When flushed, insert this bundle into the parent of the entity.
	pub fn insert_parent<R: Relationship>(bundle: impl Bundle) -> Self {
		Self::new(move |entity| {
			let Some(parent) = entity.get::<R>() else {
				bevybail!(
					"OnSpawnDeferred::new_insert_parent: Entity does not have a parent"
				);
			};
			let parent = parent.get();
			entity.world_scope(move |world| {
				world.entity_mut(parent).insert(bundle);
			});
			Ok(())
		})
	}

	/// Run all [`OnSpawnDeferred`]
	pub fn flush(
		mut commands: Commands,
		mut query: Query<(Entity, &mut Self)>,
	) {
		for (entity, mut on_spawn) in query.iter_mut() {
			commands.entity(entity).remove::<Self>();
			let func = on_spawn.take();
			commands.queue(move |world: &mut World| {
				let mut entity = world.entity_mut(entity);
				func.call(&mut entity)
			});
		}
	}

	pub fn into_command(
		self,
		entity: Entity,
	) -> impl FnOnce(&mut World) -> Result {
		move |world: &mut World| {
			let mut entity = world.entity_mut(entity);
			self.call(&mut entity)
		}
	}


	/// Call the deferred function.
	pub fn call(self, entity: &mut EntityWorldMut) -> Result {
		(self.0)(entity)
	}
	/// Convenience for getting the method from inside a system,
	/// this component should be removed when this is called
	///
	/// # Panics
	/// If the method has already been taken
	pub fn take(&mut self) -> Self {
		Self::new(std::mem::replace(
			&mut self.0,
			Box::new(|_| {
				panic!("OnSpawwnDeferred: This method has already been taken")
			}),
		))
	}
}

/// A type erased [`BundleEffect`] that runs a function when the entity is spawned.
#[derive(ImplBundle)]
pub struct CloneBundleEffect(pub Box<dyn CloneEntityFunc>);

impl CloneBundleEffect {
	/// Create a new [`OnSpawnCloneable`] effect.
	pub fn new(func: impl CloneEntityFunc) -> Self { Self(Box::new(func)) }
	/// Immediately inserts the bundle returned from this method
	pub fn insert<F, O>(func: F) -> Self
	where
		F: 'static + Send + Sync + Clone + FnOnce() -> O,
		O: Bundle,
	{
		Self::new(move |entity| {
			entity.insert(func.clone()());
		})
	}
}

impl BundleEffect for CloneBundleEffect {
	fn apply(self, entity: &mut EntityWorldMut) { (self.0)(entity); }
}

impl Clone for CloneBundleEffect {
	fn clone(&self) -> Self { Self(self.0.box_clone()) }
}

pub trait CloneEntityFunc:
	'static + Send + Sync + Fn(&mut EntityWorldMut)
{
	fn box_clone(&self) -> Box<dyn CloneEntityFunc>;
}
impl<T> CloneEntityFunc for T
where
	T: 'static + Send + Sync + Clone + Fn(&mut EntityWorldMut),
{
	fn box_clone(&self) -> Box<dyn CloneEntityFunc> { Box::new(self.clone()) }
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
	#[test]
	fn on_spawn_deferred() {
		let mut world = World::new();

		let numbers: Arc<Mutex<Vec<u32>>> = Arc::new(Mutex::new(Vec::new()));

		let numbers1 = numbers.clone();
		let numbers2 = numbers.clone();
		let numbers3 = numbers.clone();

		world.spawn((
			OnSpawnDeferred::new(move |entity_world_mut| {
				numbers1.lock().unwrap().push(1);
				entity_world_mut.insert(OnSpawn::new(move |_| {
					numbers2.lock().unwrap().push(2);
				}));
				Ok(())
			}),
			children![OnSpawnDeferred::new(move |_| {
				numbers3.lock().unwrap().push(3);
				Ok(())
			}),],
		));

		expect(&*numbers.lock().unwrap()).to_be(&[] as &[u32]);
		world.run_system_cached(OnSpawnDeferred::flush).unwrap();

		expect(&*numbers.lock().unwrap()).to_be(&[1, 2, 3]);
	}
}
