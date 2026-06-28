//! Despawn an entity automatically once a timer elapses.

use crate::prelude::*;

/// Despawns its entity once a [`Timer`] elapses, ticked by [`despawn_after`]
/// from [`Res<Time>`]. A general transient-entity helper (eg a toast that should
/// vanish after a moment); add [`despawn_after`] to drive it.
#[derive(Debug, Clone, Component)]
pub struct DespawnAfter(Timer);

impl DespawnAfter {
	/// Despawn the entity `duration` from now (a one-shot timer).
	pub fn new(duration: Duration) -> Self {
		Self(Timer::new(duration, TimerMode::Once))
	}
}

/// Tick every [`DespawnAfter`] and despawn the entities whose timer finished this
/// frame. Driven by [`Res<Time>`]; mirrors the timer-tick pattern in
/// `animate_visual_transitions`.
pub fn despawn_after(
	time: Res<Time>,
	mut commands: Commands,
	mut query: Query<(Entity, &mut DespawnAfter)>,
) {
	for (entity, mut despawn) in query.iter_mut() {
		despawn.0.tick(time.delta());
		if despawn.0.is_finished() {
			commands.entity(entity).despawn();
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use core::time::Duration;

	/// [`DespawnAfter`] keeps its entity until the duration elapses, then despawns
	/// it.
	#[beet_core::test]
	fn despawns_after_duration() {
		let mut world = World::new();
		world.init_resource::<Time>();
		let entity =
			world.spawn(DespawnAfter::new(Duration::from_secs(2))).id();
		let advance = |world: &mut World, delta: Duration| {
			world.resource_mut::<Time>().advance_by(delta);
			world.run_system_cached(despawn_after).unwrap();
		};
		// before the duration: still alive
		advance(&mut world, Duration::from_millis(1900));
		world.get_entity(entity).is_ok().xpect_true();
		// past the duration: gone
		advance(&mut world, Duration::from_millis(200));
		world.get_entity(entity).is_ok().xpect_false();
	}
}
