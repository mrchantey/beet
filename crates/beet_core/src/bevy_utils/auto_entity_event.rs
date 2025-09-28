use bevy::ecs::change_detection::MaybeLocation;
use bevy::ecs::traversal::Traversal;
use bevy::ecs::world::DeferredWorld;

use crate::prelude::*;


#[extend::ext(name=EntityWorldMutExt)]
pub impl EntityWorldMut<'_> {
	fn auto_trigger<
		'a,
		const AUTO_PROPAGATE: bool,
		E: Event<Trigger<'a> = AutoEntityTrigger<AUTO_PROPAGATE, E, T>>,
		T: 'static + Traversal<E>,
	>(
		&mut self,
		mut ev: E,
	) -> &mut Self {
		let caller = MaybeLocation::caller();
		let mut trigger =
			AutoEntityTrigger::<AUTO_PROPAGATE, E, T>::new(self.id());
		self.auto_trigger_ref(&mut ev, &mut trigger, caller);
		self
	}
	fn auto_trigger_ref<
		'a,
		const AUTO_PROPAGATE: bool,
		E: Event<Trigger<'a> = AutoEntityTrigger<AUTO_PROPAGATE, E, T>>,
		T: 'static + Traversal<E>,
	>(
		&mut self,
		ev: &mut E,
		trigger: &mut E::Trigger<'a>,
		caller: MaybeLocation,
	) -> &mut Self {
		self.world_scope(move |world| {
			let event_key = world.register_event_key::<E>();
			// SAFETY: event_key was just registered and matches `event`
			unsafe {
				DeferredWorld::from(world)
					.trigger_raw(event_key, ev, trigger, caller);
			}
		});
		self
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[derive(AutoEntityEvent)]
	struct MyEvent;

	#[test]
	fn works() {
		let mut world = World::new();
		let called = Store::default();
		world
			.spawn((
				Name::new("foo"),
				EntityObserver::new(
					move |ev: On<MyEvent>, names: Query<&Name>| {
						names
							.get(ev.trigger().event_target())
							.unwrap()
							.to_string()
							.xpect_eq("foo");
						called.set(true);
					},
				),
			))
			.auto_trigger(MyEvent);
		called.get().xpect_eq(true);
	}
}
