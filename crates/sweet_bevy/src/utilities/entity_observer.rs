use crate::effect_bundle;
use bevy::ecs::bundle::BundleEffect;
use bevy::ecs::system::IntoObserverSystem;
use bevy::prelude::*;

effect_bundle!(EntityObserver);
/// Spawn an observer for the target entity, or `Self` if no target is specified.
///
/// ## Example
///
/// This example will spawn an observer for the entity this bundle is applied to.
/// ```
/// # use sweet_bevy::prelude::*;
/// # use bevy::prelude::*;
///
/// #[derive(Event)]
/// struct Foo;
///
/// World::new().spawn(
/// 	EntityObserver::new(|_:Trigger<Foo>|{})
/// );
///
/// ```
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
}

impl BundleEffect for EntityObserver {
	fn apply(self, entity: &mut EntityWorldMut) {
		let id = self.target.unwrap_or_else(|| entity.id());
		entity.world_scope(|world| {
			world.spawn(self.observer.with_entity(id));
		});
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet_test::prelude::*;

	#[test]
	fn works() {
		#[derive(Event)]
		struct Foo(u32);

		let bucket = mock_bucket::<u32>();
		let bucket2 = bucket.clone();
		let mut world = World::new();
		world
			.spawn(EntityObserver::new(move |ev: Trigger<Foo>| {
				bucket2.call(ev.event().0)
			}))
			.trigger(Foo(3));

		expect(&bucket).to_have_returned_with(3);
	}
}
