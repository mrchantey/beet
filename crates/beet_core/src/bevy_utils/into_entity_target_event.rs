use crate::prelude::*;
use bevy::ecs::change_detection::MaybeLocation;
use bevy::ecs::event::Trigger;
use bevy::ecs::system::IntoObserverSystem;

pub trait IntoEntityTargetEvent<M>: 'static + Send + Sync {
	type Event: for<'a> Event<Trigger<'a> = Self::Trigger>;
	type Trigger: 'static + Send + Sync + Trigger<Self::Event>;

	fn into_entity_target_event(
		self,
		entity: &mut EntityWorldMut,
	) -> (Self::Event, Self::Trigger);
}

pub struct FnOnceIntoEntityTargetMarker;

impl<F, E, T> IntoEntityTargetEvent<(E, T, FnOnceIntoEntityTargetMarker)> for F
where
	F: 'static + Send + Sync + FnOnce(Entity) -> E,
	E: 'static + Send + Sync + for<'a> Event<Trigger<'a> = T>,
	T: 'static + Send + Sync + Default + Trigger<E>,
{
	type Event = E;
	type Trigger = T;

	fn into_entity_target_event(
		self,
		entity: &mut EntityWorldMut,
	) -> (Self::Event, Self::Trigger) {
		(self(entity.id()), default())
	}
}

pub struct TriggerFromTargetIntoEntityTargetMarker;


impl<E, T> IntoEntityTargetEvent<(T, TriggerFromTargetIntoEntityTargetMarker)>
	for E
where
	E: 'static + Send + Sync + for<'a> Event<Trigger<'a> = T>,
	T: 'static + Send + Sync + Trigger<E> + TriggerFromTarget,
{
	type Event = E;
	type Trigger = T;

	fn into_entity_target_event(
		self,
		entity: &mut EntityWorldMut,
	) -> (Self::Event, Self::Trigger) {
		(self, T::trigger_from_target(entity))
	}
}



pub trait TriggerFromTarget {
	fn trigger_from_target(entity: &mut EntityWorldMut) -> Self;
}


#[extend::ext(name=EntityWorldMutActionEventExt)]
pub impl EntityWorldMut<'_> {
	#[track_caller]
	fn trigger_target<M>(
		&mut self,
		ev: impl IntoEntityTargetEvent<M>,
	) -> &mut Self {
		let (mut ev, mut trigger) = ev.into_entity_target_event(self);
		let caller = MaybeLocation::caller();
		self.world_scope(move |world| {
			world.trigger_ref_with_caller_pub(&mut ev, &mut trigger, caller);
		});
		self
	}

	/// Call [`World::flush`]
	fn flush(&mut self) -> &mut Self {
		self.world_scope(|world| {
			world.flush();
		});
		self
	}
	/// Creates an [`Observer`] watching for an [`EntityEvent`] of type `E` whose [`EntityEvent::event_target`]
	/// targets this entity.
	///
	/// # Panics
	///
	/// If the entity has been despawned while this `EntityWorldMut` is still alive.
	///
	/// Panics if the given system is an exclusive system.
	// we need this because `observe` is restricted to [`EntityEvent`]
	fn observe_any<E: Event, B: Bundle, M>(
		&mut self,
		observer: impl IntoObserverSystem<E, B, M>,
	) -> &mut Self {
		// self.assert_not_despawned();
		let bundle = Observer::new(observer).with_entity(self.id());
		self.world_scope(move |world| {
			world.spawn(bundle);
		});
		self
	}
}




#[extend::ext(name=EntityCommandsActionEventExt)]
pub impl EntityCommands<'_> {
	#[track_caller]
	fn trigger_target<M>(
		&mut self,
		ev: impl IntoEntityTargetEvent<M>,
	) -> &mut Self {
		let caller = MaybeLocation::caller();
		self.queue(move |mut entity: EntityWorldMut| {
			let (mut ev, mut trigger) =
				ev.into_entity_target_event(&mut entity);
			entity.world_scope(move |world| {
				world.trigger_ref_with_caller_pub(
					&mut ev,
					&mut trigger,
					caller,
				);
			});
		});
		self
	}

	/// An [`EntityCommand`] that creates an [`Observer`](crate::observer::Observer)
	/// watching for an [`EntityEvent`] of type `E` whose [`EntityEvent::event_target`]
	/// targets this entity.
	// we need this because `observe` is restricted to [`EntityEvent`]
	#[track_caller]
	fn observe_any<E: Event, B: Bundle, M>(
		&mut self,
		observer: impl IntoObserverSystem<E, B, M>,
	) -> &mut Self {
		self.queue(move |mut entity: EntityWorldMut| {
			entity.observe_any(observer);
		});
		self
	}
}
