use crate::prelude::*;
use beet_core::prelude::*;

/// Plugin adding lifecycle management for the core beet_flow systems.
#[derive(Default)]
pub struct ControlFlowPlugin;

impl Plugin for ControlFlowPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((run_plugin::<GetOutcome>, run_plugin::<GetScore>))
			.configure_sets(Update, PreTickSet)
			.configure_sets(Update, TickSet.after(PreTickSet))
			.configure_sets(Update, PostTickSet.after(TickSet))
			.add_systems(PostStartup, get_ready_on_startup)
			.add_systems(
				Update,
				(
					(
						// flush any triggers spawned by TriggerDeferred
						OnSpawnDeferred::flush,
						tick_run_timers,
					)
						.in_set(PreTickSet),
					end_in_duration::<Outcome>.in_set(TickSet),
				),
			)
			.add_observer(reset_run_time_started)
			.add_observer(reset_run_timer_stopped);
	}
}

/// Any [RunTimer] will be ticked, runs before [`TickSet`]
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct PreTickSet;

/// The set in which most actions that use [`Running`] should be run.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct TickSet;

/// Runs after [`TickSet`]
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct PostTickSet;



/// This plugin should be registered for any [`RunEvent`] and [`EndEvent`] pair,
/// ensuring events are properly propagated and interrupted.
pub fn run_plugin<R: RunEvent>(app: &mut App) {
	app.add_observer(interrupt_on_run::<R>);
	app.add_observer(interrupt_on_end::<R::End>);
	app.add_observer(propagate_end::<R::End>);
	app.add_observer(propagate_child_end::<R::End>);
}


/// Actions can take many forms, these tags help categorize them.
/// The convention is to add these in a list just after the description
/// of the action, and before the example:
/// ```
/// # use beet_flow::prelude::*;
/// /// ## Tags
/// /// - [LongRunning](ActionTag::LongRunning)
/// /// - [MutateOrigin](ActionTag::MutateOrigin)
/// struct MyAction;
/// ```
pub enum ActionTag {
	/// Actions concerned with control flow, usually
	/// triggering [GetOutcome] and [Outcome] events.
	ControlFlow,
	/// Actions that use the [Running] component to run
	/// over multiple frames.
	LongRunning,
	/// This action makes global changes to the world.
	MutateWorld,
	/// This action makes changes to the [`origin`](OnRun::origin] entity.
	MutateOrigin,
	/// This action is concerned with providing output to the user or
	/// receiving input.
	InputOutput,
}


/// Helper to collect all [`Run`] calls, storing their [`Name`] or "Unknown" if missing
pub fn collect_on_run(world: &mut World) -> Store<Vec<String>> {
	let store = Store::default();
	world.add_observer(move |ev: On<GetOutcome>, query: Query<&Name>| {
		let name = if let Ok(name) = query.get(ev.target()) {
			name.to_string()
		} else {
			"Unknown".to_string()
		};
		store.push(name);
	});
	store
}

/// Collect all [`Outcome`] with a [`Name`]
pub fn collect_on_result(world: &mut World) -> Store<Vec<(String, Outcome)>> {
	let store = Store::default();
	world.add_observer(move |ev: On<Outcome>, query: Query<&Name>| {
		let name = if let Ok(name) = query.get(ev.target()) {
			name.to_string()
		} else {
			"Unknown".to_string()
		};
		store.push((name, ev.clone()));
	});
	store
}
