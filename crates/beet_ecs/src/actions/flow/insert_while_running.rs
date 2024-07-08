// use crate::prelude::*;
// use bevy::prelude::*;

// #[derive(Default, Action, Reflect)]
// #[reflect(Default, Component)]
// pub struct InsertWhileRunning<T>(pub T);


// fn on_start_running<T: Component>(
// 	trigger: Trigger<OnRun>,
// 	query: Query<&T>,
// 	mut commands: Commands,
// ) {
// 	commands.entity(trigger.entity()).insert(Running);
// }
// fn on_stop_running(
// 	trigger: Trigger<OnRunResult>,
// 	mut commands: Commands,
// 	query: Query<&Children>,
// ) {
// 	if let Ok(children) = query.get(trigger.entity()) {
// 		if let Some(first_child) = children.iter().next() {
// 			commands.trigger_targets(OnRun, *first_child);
// 		}
// 	}
// }
