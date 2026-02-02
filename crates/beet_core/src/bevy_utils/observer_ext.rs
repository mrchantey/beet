//! Observer extension functions for testing.
//!
//! This module provides helper functions for observing and collecting
//! triggered events during tests.

use crate::prelude::*;

/// Observes all triggers of type `E` and collects them into a [`Store`].
///
/// This is useful for testing that events are triggered correctly.
pub fn observe_triggers<E: Event + Clone + Send + 'static>(
	world: &mut World,
) -> Store<Vec<E>> {
	let store = Store::default();
	world.add_observer(move |on_result: On<E>| {
		store.push(on_result.event().clone());
	});
	store
}

/// Observes all entity triggers of type `E` and collects target entity names into a [`Store`].
///
/// This is useful for testing that events target the correct entities.
pub fn observe_trigger_names<E: EntityEvent + Send + 'static>(
	world: &mut World,
) -> Store<Vec<String>> {
	let store = Store::default();
	world.add_observer(move |on_result: On<E>, query: Query<&Name>| {
		if let Ok(name) = query.get(on_result.event().event_target()) {
			store.push(name.to_string());
		}
	});
	store
}
