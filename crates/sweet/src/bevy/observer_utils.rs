use beet_utils::prelude::*;
use bevy::prelude::*;

pub fn observe_triggers<E: Event + Clone + Send + 'static>(
	world: &mut World,
) -> Store<Vec<E>> {
	let store = Store::default();
	world.add_observer(move |on_result: Trigger<E>| {
		store.push(on_result.event().clone());
	});
	store
}

pub fn observe_trigger_names<E: Event + Send + 'static>(
	world: &mut World,
) -> Store<Vec<String>> {
	let store = Store::default();
	world.add_observer(move |on_result: Trigger<E>, query: Query<&Name>| {
		if let Ok(name) = query.get(on_result.target()) {
			store.push(name.to_string());
		}
	});
	store
}
