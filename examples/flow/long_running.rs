//! Sometimes we want an action to run for multiple frames.
//! In Beet the continuous action to perform is usually seperate from the
//! Terminating factor.
//!
//! The below example includes a `Patrol Sequence` which will run indefinitely.
//! and uses [TriggerInDuration] to end the behavior after 1 second.
//!
//! For a distance based trigger see [EndOnArrive].
//!
//! Note that long running terminators should require [ContinueRun]
//! which sets up the [Running] component lifecycle.
#![allow(unused)]
use beet::prelude::*;
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use std::time::Duration;


// #[rustfmt::skip]
fn main() {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		BeetFlowPlugin::default(),
		DebugFlowPlugin::with_result(),
	))
	.add_systems(Update, patrol.run_if(on_timer(Duration::from_millis(100))));

	app.world_mut()
		.spawn((Name::new("root"), Sequence))
		.with_children(|parent| {
			parent
				.spawn((
					Name::new("Long Running"),
					Sequence,
					// this is the end condition, triggering OnRunResult::success() after a duration
					EndInDuration::new(
						Outcome::Pass,
						Duration::from_secs(5),
					),
				))
				.with_children(|parent| {
					// we need a nested sequence so that `RepeatFlow` is scoped
					// *under* the trigger so it can be properly iterrupted,
					// otherwise `Long Running` will just start again
					parent
						.spawn((
							Name::new("Patrol Sequence"),
							Sequence,
							// the patrol sequence will repeat indefinitely
							Repeat::default(),
						))
						.with_child((
							// patrol the left flank for a bit
							Name::new("Patrol Left"),
							Patrol::default(),
							EndInDuration::new(
								Outcome::Pass,
								Duration::from_secs(1),
							),
						))
						.with_child((
							// patrol the right flank for a bit
							Name::new("Patrol Right"),
							Patrol::default(),
							EndInDuration::new(
								Outcome::Pass,
								Duration::from_secs(1),
							),
						));
				});
			parent.spawn(Name::new("After Long Running")).observe(
				|_trigger: On<GetOutcome>| {
					println!("After Long Running triggered, exiting");
					std::process::exit(0);
				},
			);
		})
		.trigger_target(GetOutcome);

	app.run();
}


#[derive(Default, Component, Reflect)]
// any action that uses the [`Running`] component should require [`ContinueRun`]
#[require(ContinueRun)]
struct Patrol {
	count: usize,
}

fn patrol(mut query: Query<(&mut Patrol, &Name), With<Running>>) {
	for (mut action, name) in query.iter_mut() {
		action.count += 1;
		println!("{}: {}", name, action.count);
	}
}
