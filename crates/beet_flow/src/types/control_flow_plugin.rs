//! Core plugin and system sets for beet_flow lifecycle management.
use crate::prelude::*;
use beet_core::prelude::*;

/// Plugin that manages action lifecycles and event propagation.
///
/// This plugin registers the core systems and observers needed for beet_flow
/// to function, including timer ticking, duration-based endings, and the
/// default [`GetOutcome`]/[`Outcome`] and [`GetScore`]/[`Score`] event pairs.
///
/// # Example
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// App::new()
///     .add_plugins(ControlFlowPlugin::default());
/// ```
#[derive(Default)]
pub struct ControlFlowPlugin;

impl Plugin for ControlFlowPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((run_plugin::<GetOutcome>, run_plugin::<GetScore>))
			.configure_sets(Update, PreTickSet)
			.configure_sets(Update, TickSet.after(PreTickSet))
			.configure_sets(Update, PostTickSet.after(TickSet))
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

/// System set that runs before [`TickSet`].
///
/// Used for setup work like flushing deferred triggers and ticking timers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct PreTickSet;

/// The primary system set for actions that use [`Running`].
///
/// Most long-running action systems should be placed in this set.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct TickSet;

/// System set that runs after [`TickSet`].
///
/// Used for cleanup and logging that should occur after actions have ticked.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct PostTickSet;



/// Registers lifecycle observers for a custom [`RunEvent`]/[`EndEvent`] pair.
///
/// This plugin should be registered for any custom event pair to ensure
/// events are properly propagated and running actions are interrupted.
///
/// # Example
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// // Define custom run/end events
/// #[derive(Debug, Default, Clone, EntityTargetEvent)]
/// struct MyRun;
///
/// #[derive(Debug, Clone, EntityTargetEvent)]
/// struct MyEnd(bool);
///
/// impl RunEvent for MyRun {
///     type End = MyEnd;
/// }
///
/// impl EndEvent for MyEnd {
///     type Run = MyRun;
/// }
///
/// // Register the plugin
/// App::new()
///     .add_plugins(run_plugin::<MyRun>);
/// ```
pub fn run_plugin<R: RunEvent>(app: &mut App) {
	app.add_observer(interrupt_on_run::<R>);
	app.add_observer(interrupt_on_end::<R::End>);
	app.add_observer(propagate_end::<R::End>);
	app.add_observer(propagate_child_end::<R::End>);
}


/// Documentation-only enum describing action categories.
///
/// Use these tags in doc comments to categorize actions:
///
/// ```
/// # use beet_flow::prelude::*;
/// /// ## Tags
/// /// - [LongRunning](ActionTag::LongRunning)
/// /// - [MutateAgent](ActionTag::MutateAgent)
/// struct MyAction;
/// ```
// TODO i think we can replace this with bev_reflect trait impls
pub enum ActionTag {
	/// Actions concerned with control flow, usually
	/// triggering [`GetOutcome`] and [`Outcome`] events.
	ControlFlow,
	/// Actions that use the [`Running`] component to run
	/// over multiple frames.
	LongRunning,
	/// This action makes global changes to the world.
	MutateWorld,
	/// This action makes changes to the agent entity.
	MutateAgent,
	/// This action is concerned with providing output to the user or
	/// receiving input.
	InputOutput,
}


/// Collects all [`GetOutcome`] triggers, storing the [`Name`] of each target.
///
/// Returns a [`Store`] containing entity names (or "Unknown" if no [`Name`]).
/// Useful for testing and debugging action execution order.
///
/// # Example
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// let mut world = ControlFlowPlugin::world();
/// let on_run = collect_on_run(&mut world);
///
/// world
///     .spawn((Name::new("my_action"), EndWith(Outcome::Pass)))
///     .trigger_target(GetOutcome)
///     .flush();
///
/// assert_eq!(on_run.get(), vec!["my_action".to_string()]);
/// ```
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

/// Collects all [`Outcome`] triggers with their entity [`Name`].
///
/// Returns a [`Store`] of `(name, outcome)` tuples. Useful for testing
/// action results and their order.
///
/// # Example
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// let mut world = ControlFlowPlugin::world();
/// let on_result = collect_on_result(&mut world);
///
/// world
///     .spawn((Name::new("my_action"), EndWith(Outcome::Pass)))
///     .trigger_target(GetOutcome)
///     .flush();
///
/// assert_eq!(
///     on_result.get(),
///     vec![("my_action".to_string(), Outcome::Pass)]
/// );
/// ```
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
