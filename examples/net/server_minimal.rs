//! A minimal server example
use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			ServerPlugin::default(),
		))
		.add_observer(|ev: On<Insert, Request>, mut commands: Commands| {
			commands
				.entity(ev.event_target())
				.insert(Response::ok_body("hello world", "text/plain"));
		})
		.run();
}
