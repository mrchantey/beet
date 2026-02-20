//! On-spawn effects for Bevy entities.
//!
//! This module provides bundle effects that run when an entity is spawned,
//! allowing for deferred initialization, dynamic bundle insertion, and
//! entity relationship setup.
//!
//! # Key Types
//!
//! - [`OnSpawn`] - Type-erased effect that runs immediately when spawned
//! - [`OnSpawnTyped`] - Generic version that preserves the closure type
//! - [`OnSpawnDeferred`] - Effect that runs when explicitly flushed
//! - [`OnSpawnClone`] - Cloneable version of [`OnSpawn`]

use crate::prelude::*;
use beet_core_macros::BundleEffect;
use bevy::ecs::error::ErrorContext;
use bevy::ecs::relationship::RelatedSpawner;
use bevy::ecs::relationship::Relationship;
use bevy::ecs::spawn::SpawnRelatedBundle;
use bevy::ecs::spawn::SpawnWith;
use bevy::ecs::system::IntoObserverSystem;

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


/// A type-erased [`BundleEffect`] that runs a function when the entity is spawned.
///
/// This is the most flexible spawn effect, accepting any boxed closure.
/// For a typed version, see [`OnSpawnTyped`].
#[derive(BundleEffect)]
pub struct OnSpawn(
	pub Box<dyn 'static + Send + Sync + FnOnce(&mut EntityWorldMut)>,
);

/// Trait for allowing bundles and results to be returned from methods.
pub trait ApplyToEntity<M>: 'static + Send + Sync {
	/// Applies this value to the entity.
	fn apply(self, entity: &mut EntityWorldMut);
}

/// Marker type for bundle implementations of [`ApplyToEntity`].
pub struct BundleApplyToEntityMarker;

/// Marker type for result implementations of [`ApplyToEntity`].
pub struct ResultApplyToEntityMarker;

impl<T: Bundle> ApplyToEntity<BundleApplyToEntityMarker> for T {
	fn apply(self, entity: &mut EntityWorldMut) { entity.insert(self); }
}

impl<T: Bundle> ApplyToEntity<ResultApplyToEntityMarker> for Result<T> {
	fn apply(self, entity: &mut EntityWorldMut) {
		match self {
			Ok(bundle) => {
				entity.insert(bundle);
			}
			Err(err) => entity.world_scope(|world| {
				world.default_error_handler()(
					err.into(),
					ErrorContext::Command {
						name: "ApplyToEntity".into(),
					},
				);
			}),
		}
	}
}
impl OnSpawn {
	/// Creates a new [`OnSpawn`] effect.
	pub fn new(
		func: impl 'static + Send + Sync + FnOnce(&mut EntityWorldMut),
	) -> Self {
		Self(Box::new(func))
	}

	/// Inserts a bundle into the entity on spawn.
	pub fn insert(bundle: impl Bundle) -> Self {
		Self::new(move |entity| {
			entity.insert(bundle);
		})
	}
	/// Inserts a bundle into the entities children on spawn,
	/// avoiding bevy's duplicate component gotya with children!
	pub fn insert_child(bundle: impl Bundle) -> Self {
		Self::new(move |entity| {
			let id = entity.id();
			entity.world_scope(move |world| {
				world.spawn((bundle, ChildOf(id)));
			});
		})
	}

	/// Inserts the bundle if it is `Some`.
	pub fn insert_option(bundle: Option<impl Bundle>) -> Self {
		Self::new(move |entity| {
			if let Some(bundle) = bundle {
				entity.insert(bundle);
			}
		})
	}

	/// Runs the system and inserts the resulting bundle into the entity on spawn.
	pub fn run_insert<
		System: 'static + Send + Sync + IntoSystem<In<Entity>, Out, M1>,
		M1,
		Out: ApplyToEntity<M2>,
		M2,
	>(
		system: System,
	) -> Self {
		Self::new(move |entity| {
			let id = entity.id();
			entity
				.world_scope(move |world| {
					world.run_system_once_with(system, id)
				})
				.unwrap()
				.apply(entity);
		})
	}

	/// Inserts the resource into the world when the entity is spawned.
	pub fn insert_resource(resource: impl Resource) -> Self {
		Self::new(move |entity| {
			entity.world_scope(move |world| world.insert_resource(resource));
		})
	}

	/// Triggers an entity target event on spawn.
	pub fn trigger<M>(event: impl IntoEntityTargetEvent<M>) -> Self {
		Self::new(move |entity| {
			entity.trigger_target(event);
		})
	}

	/// Triggers an entity target event on spawn if the event is `Some`.
	pub fn trigger_option<M>(
		event: Option<impl IntoEntityTargetEvent<M>>,
	) -> Self {
		Self::new(move |entity| {
			if let Some(event) = event {
				entity.trigger_target(event);
			}
		})
	}

	/// Registers an observer on the entity on spawn.
	pub fn observe<E: Event, B: Bundle, M>(
		observer: impl 'static + Send + Sync + IntoObserverSystem<E, B, M>,
	) -> Self {
		Self::new(move |entity| {
			entity.observe_any(observer);
		})
	}


	fn effect(self, entity: &mut EntityWorldMut) { (self.0)(entity); }

	/// Creates a new [`OnSpawn`] effect that runs an async function.
	pub fn new_async<Fut, Out>(
		func: impl 'static + Send + Sync + FnOnce(AsyncEntity) -> Fut,
	) -> Self
	where
		Fut: 'static + Send + Sync + Future<Output = Out>,
		Out: 'static + AsyncTaskOut,
	{
		Self(Box::new(move |entity| {
			let id = entity.id();
			entity.world_scope(move |world| {
				world
					.run_async(async move |world| func(world.entity(id)).await);
			});
		}))
	}

	/// Creates a new [`OnSpawn`] effect that runs an async function on the local thread.
	pub fn new_async_local<Fut, Out>(
		func: impl 'static + Send + Sync + FnOnce(AsyncEntity) -> Fut,
	) -> Self
	where
		Fut: 'static + Future<Output = Out>,
		Out: 'static + AsyncTaskOut,
	{
		Self(Box::new(move |entity| {
			let id = entity.id();
			entity.world_scope(move |world| {
				world.run_async_local(async move |world| {
					func(world.entity(id)).await
				});
			});
		}))
	}
}


/// A [`BundleEffect`] that runs a typed function when the entity is spawned.
///
/// Unlike [`OnSpawn`], this preserves the closure type for better type inference.
#[derive(Clone, BundleEffect)]
pub struct OnSpawnTyped<F: 'static + Send + Sync + FnOnce(&mut EntityWorldMut)>(
	pub F,
);

impl<F: Send + Sync + FnOnce(&mut EntityWorldMut)> OnSpawnTyped<F> {
	/// Creates a new [`OnSpawnTyped`] effect.
	pub fn new(func: F) -> Self { Self(func) }

	fn effect(self, entity: &mut EntityWorldMut) { self.0(entity); }
}

/// A component that runs a deferred function when explicitly flushed.
///
/// Unlike [`OnSpawn`], this does not run immediately when spawned.
/// Instead, it must be flushed by running the [`OnSpawnDeferred::flush`] system.
#[derive(Component)]
pub struct OnSpawnDeferred(
	pub Box<dyn 'static + Send + Sync + FnOnce(&mut EntityWorldMut) -> Result>,
);

impl OnSpawnDeferred {
	/// Creates a new [`OnSpawnDeferred`] effect.
	pub fn new(
		func: impl 'static + Send + Sync + FnOnce(&mut EntityWorldMut) -> Result,
	) -> Self {
		Self(Box::new(func))
	}

	/// Runs the function for the parent of this entity.
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

	/// Inserts this bundle into the entity when flushed.
	pub fn insert(bundle: impl Bundle) -> Self {
		Self::new(move |entity| {
			entity.insert(bundle);
			Ok(())
		})
	}

	/// When flushed, inserts this bundle into the parent of the entity.
	pub fn insert_parent<R: Relationship>(bundle: impl Bundle) -> Self {
		Self::parent::<R>(move |entity| {
			entity.insert(bundle);
			Ok(())
		})
	}

	/// Triggers an entity target event when flushed.
	pub fn trigger_target<M>(ev: impl IntoEntityTargetEvent<M>) -> Self {
		Self::new(move |entity| {
			entity.trigger_target(ev);
			Ok(())
		})
	}

	/// System that runs all [`OnSpawnDeferred`] components.
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

	/// Converts this into a command for the given entity.
	pub fn into_command(
		self,
		entity: Entity,
	) -> impl FnOnce(&mut World) -> Result {
		move |world: &mut World| {
			let mut entity = world.entity_mut(entity);
			self.call(&mut entity)
		}
	}


	/// Calls the deferred function.
	pub fn call(self, entity: &mut EntityWorldMut) -> Result {
		(self.0)(entity)
	}

	/// Takes the method from this component.
	///
	/// This component should be removed when this is called.
	///
	/// # Panics
	///
	/// Panics if the method has already been taken.
	pub fn take(&mut self) -> Self {
		Self::new(std::mem::replace(
			&mut self.0,
			Box::new(|_| {
				panic!("OnSpawwnDeferred: This method has already been taken")
			}),
		))
	}
}

/// A [`Clone`]able version of [`OnSpawn`].
///
/// Uses a trait object that implements [`Clone`] to allow the effect to be cloned.
#[derive(BundleEffect)]
pub struct OnSpawnClone(pub Box<dyn CloneEntityFunc>);

impl OnSpawnClone {
	/// Creates a new [`OnSpawnClone`] effect.
	pub fn new(func: impl CloneEntityFunc) -> Self { Self(Box::new(func)) }

	/// Creates an effect that immediately inserts the bundle returned from the closure.
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

/// Trait for cloneable entity functions.
///
/// This is used by [`OnSpawnClone`] to allow cloning of the inner closure.
pub trait CloneEntityFunc:
	'static + Send + Sync + Fn(&mut EntityWorldMut)
{
	/// Creates a boxed clone of this function.
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

	#[test]
	fn dfs() {
		let mut world = World::new();

		let numbers = Store::default();

		world.spawn((
			OnSpawnTyped::new(move |entity_world_mut| {
				numbers.push(1);
				entity_world_mut.insert(OnSpawnTyped::new(move |_| {
					numbers.push(2);
				}));
			}),
			OnSpawnTyped::new(move |_| {
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
				entity_world_mut.insert(OnSpawnTyped::new(move |_| {
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

		// why is this?
		#[cfg(target_arch = "wasm32")]
		numbers.get().xpect_eq(&[1, 2, 3]);
		#[cfg(not(target_arch = "wasm32"))]
		numbers.get().xpect_eq(&[3, 1, 2]);
	}

	#[test]
	fn observe() {
		#[derive(EntityEvent)]
		struct Foo(Entity);

		let store = Store::default();
		let mut world = World::new();
		world
			.spawn(OnSpawn::observe(move |_: On<Foo>| store.set(3)))
			.trigger(Foo);

		store.get().xpect_eq(3);
	}
}
