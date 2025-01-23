//! Sometimes we want an action to run for multiple frames.
//! In Beet the continuous action to perform is usually seperate from the
//! Terminating factor.
//!
//! The below example includes [DoPushup] which will run indefinitely.
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


#[rustfmt::skip]
fn main() {
	let mut app = App::new();
	app.insert_resource(BeetDebugConfig::default())
		.add_plugins((
			MinimalPlugins,
			BeetDefaultPlugins,
			BeetDebugPlugin,
			bevy::log::LogPlugin::default(),
			ActionPlugin::<DoPushup>::default(),
		));

	app.world_mut()
		.spawn((
			Name::new("root"), 
			SequenceFlow
		))
		.with_children(|parent|{
			parent.spawn((
				Name::new("Long Running"),
				// this is the long-running action
				DoPushup::default(),
				// and this is the end condition, triggering OnRunResult::success() after 1 second
				TriggerInDuration::new(
					OnRunResult::success(),
					Duration::from_secs(1)
				),
			));
			parent.spawn((
				Name::new("Child 2"), 
			)).observe(|_trigger:Trigger<OnRun>|{
				println!("Child 2 triggered, exiting");
				std::process::exit(0);
			});
		}).trigger(OnRun);

	app.run();
}


#[derive(Default, Component, Action, Reflect)]
#[systems(do_pushup.run_if(on_timer(Duration::from_millis(100))))]
struct DoPushup(usize);


fn do_pushup(mut query: Query<&mut DoPushup, With<Running>>) {
	for mut pushup in query.iter_mut() {
		pushup.0 += 1;
		println!("Phew, doing pushup number: {}", pushup.0);
	}
}
