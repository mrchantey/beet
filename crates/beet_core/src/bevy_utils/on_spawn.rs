use crate::prelude::*;
use beet_core_macros::BundleEffect;
use bevy::ecs::relationship::RelatedSpawner;
use bevy::ecs::relationship::Relationship;
use bevy::ecs::spawn::SpawnRelatedBundle;
use bevy::ecs::spawn::SpawnWith;
use bevy::ecs::traversal::Traversal;

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


/// A [`BundleEffect`] that runs a function when the entity is spawned.
#[derive(Clone, BundleEffect)]
pub struct OnSpawn<F: 'static + Send + Sync + FnOnce(&mut EntityWorldMut)>(
	pub F,
);

impl<F: Send + Sync + FnOnce(&mut EntityWorldMut)> OnSpawn<F> {
	/// Create a new [`OnSpawn`] effect.
	pub fn new(func: F) -> Self { Self(func) }

	fn effect(self, entity: &mut EntityWorldMut) { self.0(entity); }
}

/// A type erased [`BundleEffect`] that runs a function when the entity is spawned.
#[derive(BundleEffect)]
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
	/// Insert this bundle into the entity on spawn.
	pub fn insert(bundle: impl Bundle) -> Self {
		Self::new(move |entity| {
			entity.insert(bundle);
		})
	}
	/// Insert the bundle if it is `Some`
	pub fn insert_option(bundle: Option<impl Bundle>) -> Self {
		Self::new(move |entity| {
			if let Some(bundle) = bundle {
				entity.insert(bundle);
			}
		})
	}

	pub fn insert_resource(resource: impl Resource) -> Self {
		Self::new(move |entity| {
			entity.world_scope(move |world| world.insert_resource(resource));
		})
	}

	pub fn trigger<
		'a,
		const AUTO_PROPAGATE: bool,
		E: Event<Trigger<'a> = EventTargetTrigger<AUTO_PROPAGATE, E, T>>,
		T: 'static + Traversal<E>,
	>(
		ev: E,
	) -> Self {
		Self::new(move |entity| {
			entity.trigger_target(ev);
		})
	}
	pub fn trigger_option<
		'a,
		const AUTO_PROPAGATE: bool,
		E: Event<Trigger<'a> = EventTargetTrigger<AUTO_PROPAGATE, E, T>>,
		T: 'static + Traversal<E>,
	>(
		ev: Option<E>,
	) -> Self {
		Self::new(move |entity| {
			if let Some(ev) = ev {
				entity.trigger_target(ev);
			}
		})
	}
	fn effect(self, entity: &mut EntityWorldMut) { (self.0)(entity); }
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

	/// Run the function for the parent of this entity
	pub fn parent<R: Relationship>(
		func: impl 'static + Send + Sync + FnOnce(&mut EntityWorldMut) -> Result,
	) -> Self {
		Self::new(move |entity| {
			let Some(parent) = entity.get::<R>() else {
				bevybail!(
					"OnSpawnDeferred::insert_parent: Entity does not have a parent"
				);
			};
			let parent = parent.get();
			entity.world_scope(move |world| func(&mut world.entity_mut(parent)))
		})
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
		Self::parent::<R>(move |entity| {
			entity.insert(bundle);
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

/// A [`Clone`] version of [`OnSpawnBoxed`]
#[derive(BundleEffect)]
pub struct OnSpawnClone(pub Box<dyn CloneEntityFunc>);

impl OnSpawnClone {
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
	fn effect(self, entity: &mut EntityWorldMut) { (self.0)(entity); }
}


impl Clone for OnSpawnClone {
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
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn dfs() {
		let mut world = World::new();

		let numbers = Store::default();

		world.spawn((
			OnSpawn::new(move |entity_world_mut| {
				numbers.push(1);
				entity_world_mut.insert(OnSpawn::new(move |_| {
					numbers.push(2);
				}));
			}),
			OnSpawn::new(move |_| {
				numbers.push(3);
			}),
		));

		numbers.get().xpect_eq(&[1, 2, 3]);
	}
	#[test]
	fn on_spawn_deferred() {
		let mut world = World::new();

		let numbers = Store::default();
		world.spawn((
			OnSpawnDeferred::new(move |entity_world_mut| {
				numbers.push(1);
				entity_world_mut.insert(OnSpawn::new(move |_| {
					numbers.push(2);
				}));
				Ok(())
			}),
			children![OnSpawnDeferred::new(move |_| {
				numbers.push(3);
				Ok(())
			}),],
		));

		numbers.get().xpect_eq(&[] as &[u32]);
		world.run_system_cached(OnSpawnDeferred::flush).unwrap();

		numbers.get().xpect_eq(&[3, 1, 2]);
	}
}
