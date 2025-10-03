use bevy::ecs::change_detection::MaybeLocation;

use crate::prelude::*;


/// A type that, given a target [`Entity`] can be converted
/// into an [`EntityEvent`].
/// This is useful for non-default events that cannot
/// be created via `From::Entity`
pub trait EventPayload
where
	Self: 'static + Send + Sync,
	for<'t> Self::Event: EntityEvent<Trigger<'t>: Default>,
{
	type Event;
	fn into_event(self, entity: Entity) -> Self::Event;
}
impl<T> EventPayload for T
where
	T: FnOnce(Entity) -> Self,
	Self: 'static + Send + Sync,
	for<'t> Self: EntityEvent<Trigger<'t>: Default>,
{
	type Event = Self;
	fn into_event(self, entity: Entity) -> Self::Event { (self)(entity) }
}


pub struct EntityEventFunc<T: From<Entity>> {
	phantom: PhantomData<T>,
}

impl<T: From<Entity>> Default for EntityEventFunc<T> {
	fn default() -> Self { Self { phantom: default() } }
}
impl<T: From<Entity>> EntityEventFunc<T> {
	pub fn create(&self, entity: Entity) -> T { entity.into() }
}

impl<T: From<Entity>> Clone for EntityEventFunc<T> {
	fn clone(&self) -> Self { Self::default() }
}


#[extend::ext(name=EntityCommandsEventPayloadExt)]
pub impl EntityCommands<'_> {
	fn trigger_payload<T: EventPayload>(&mut self, ev: T) -> &mut Self {
		let caller = MaybeLocation::caller();
		let mut event = ev.into_event(self.id());
		self.queue(move |mut entity: EntityWorldMut| {
			entity.world_scope(|world| {
				world.trigger_ref_with_caller_pub(
					&mut event,
					&mut <<T::Event as Event>::Trigger<'_> as Default>::default(
					),
					caller,
				);
			});
		});
		self
	}
}

#[extend::ext(name=EntityWorldMutEventPayloadExt)]
pub impl EntityWorldMut<'_> {
	fn trigger_payload<T: EventPayload>(&mut self, ev: T) -> &mut Self {
		let caller = MaybeLocation::caller();
		let mut event = ev.into_event(self.id());
		self.world_scope(|world| {
			world.trigger_ref_with_caller_pub(
				&mut event,
				&mut <<T::Event as Event>::Trigger<'_> as Default>::default(),
				caller,
			);
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
}
