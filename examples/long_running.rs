//! Sometimes we want an action to run for multiple frames.
//! In Beet the continuous action to perform is usually seperate from the
//! Terminating factor.
//!
//! The below example includes [DoExercise] which will run indefinitely.
//! and uses [TriggerInDuration] to end the behavior after 1 second.
//!
//! For a distance based trigger see [EndOnArrive].
//!
//! Note that long running terminators should require [ContinueRun]
//! which sets up the [Running] component lifecycle.
use beet::prelude::*;
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use std::time::Duration;


// #[rustfmt::skip]
fn main() {
	let mut app = App::new();
	app.insert_resource(BeetDebugConfig::default())
		.add_plugins((
			MinimalPlugins,
			BeetDefaultPlugins,
			BeetDebugPlugin,
			bevy::log::LogPlugin::default(),
			ActionPlugin::<Patrol>::default(),
		));

	app.world_mut()
		.spawn((Name::new("root"), SequenceFlow))
		.with_children(|parent| {
			parent
				.spawn((
					Name::new("Long Running"),
					// this is the long-running action
					SequenceFlow,
					// and this is the end condition, triggering OnRunResult::success() after 1 second
					TriggerInDuration::new(
						OnRunResult::success(),
						Duration::from_secs(1),
					),
				))
				.with_children(|parent| {
					// we need a nested sequence so that repeat is scoped
					// *under* the trigger, so it will be interrupted.
					parent
						.spawn((
							Name::new("Patrol Sequence"),
							SequenceFlow,
							RepeatFlow::default(),
						))
						.with_child((
							Name::new("Patrol Left"),
							Patrol::new("Patrol Left Terrace: "),
							TriggerInDuration::new(
								OnRunResult::success(),
								Duration::from_millis(300),
							),
						))
						.with_child((
							Name::new("Patrol Right"),
							Patrol::new("Patrol Right Terrace: "),
							TriggerInDuration::new(
								OnRunResult::success(),
								Duration::from_millis(300),
							),
							// TriggerOnTrigger::<OnRun, OnRunResult>::new(OnRun)
							// 	.with_target(parent.parent_entity()),
						));
				});
			parent.spawn((Name::new("Child 2"),)).observe(
				|_trigger: Trigger<OnRun>| {
					println!("Child 2 triggered, exiting");
					// std::process::exit(0);
				},
			);
		})
		.trigger(OnRun);

	app.run();
}


#[derive(Default, Component, Action, Reflect)]
#[systems(patrol.run_if(on_timer(Duration::from_millis(100))))]
// any action that uses the [`Running`] component should require [`ContinueRun`]
#[require(ContinueRun)]
struct Patrol {
	prefix: String,
	count: usize,
}
impl Patrol {
	pub fn new(prefix: &str) -> Self {
		Self {
			prefix: prefix.to_string(),
			count: 0,
		}
	}
}


fn patrol(mut query: Query<&mut Patrol, With<Running>>) {
	for mut action in query.iter_mut() {
		action.count += 1;
		println!("{}{}", action.prefix, action.count);
	}
}
