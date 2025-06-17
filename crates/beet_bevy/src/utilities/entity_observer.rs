use beet_common_macros::ImplBundle;
use bevy::ecs::bundle::BundleEffect;
use bevy::ecs::system::IntoObserverSystem;
use bevy::prelude::*;

/// Spawn an observer for the target entity, or `Self` if no target is specified.
///
/// ## Example
///
/// This example will spawn an observer for the entity this bundle is applied to.
/// ```
/// # use beet_bevy::prelude::*;
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
#[derive(ImplBundle)]
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
		// If no target is specified, use the entity this bundle is applied to.
		let target = self.target.unwrap_or_else(|| entity.id());
		let observed_entity = entity.world_scope(|world| {
			world.spawn(self.observer.with_entity(target)).id()
		});
		entity.insert(SpawnedEntityObserver {
			observer: observed_entity,
		});
	}
}

#[derive(Debug, Clone, Copy, Component)]
pub struct SpawnedEntityObserver {
	pub observer: Entity,
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

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
