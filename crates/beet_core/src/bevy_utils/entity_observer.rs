use crate::prelude::*;
use bevy::ecs::system::IntoObserverSystem;

/// Spawn an observer for the target entity, or `Self` if no target is specified.
///
/// ## Example
///
/// This example will spawn an observer for the entity this bundle is applied to.
/// ```
/// # use beet_core::prelude::*;
/// # ///
/// #[derive(Event)]
/// struct Foo;
///
/// World::new().spawn(
/// 	EntityObserver::new(|_:Trigger<Foo>|{})
/// );
///
/// ```
#[derive(BundleEffect)]
pub struct EntityObserver {
	/// The observer to spawn.
	observer: Observer,
	/// The target entity to observe, leave blank to observe the entity this bundle
	/// is applied to.
	target: Option<Entity>,
}

impl EntityObserver {
	pub fn new<E: Event, B: Bundle, M>(
		observer: impl IntoObserverSystem<E, B, M>,
	) -> Self {
		Self {
			observer: Observer::new(observer),
			target: None,
		}
	}

	pub fn with_entity(mut self, target: Entity) -> Self {
		self.target = Some(target);
		self
	}
	fn effect(self, entity: &mut EntityWorldMut) {
		// If no target is specified, use the entity this bundle is applied to.
		let target = self.target.unwrap_or_else(|| entity.id());
		entity.world_scope(|world| {
			world.spawn(self.observer.with_entity(target));
		});
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		#[derive(EntityTargetEvent)]
		struct Foo(u32);

		let store = Store::default();
		let mut world = World::new();
		world
			.spawn(EntityObserver::new(move |ev: On<Foo>| {
				store.set(ev.event().0)
			}))
			.trigger_target(Foo(3));

		store.get().xpect_eq(3);
	}
}
