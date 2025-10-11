//! A minimal server example
use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			FlowRouterPlugin::default(),
		))
		.add_systems(Startup, |mut commands: Commands| {
			commands.spawn((RouteServer, InfallibleSequence, children![
				endpoint(
					HttpMethod::Get,
					handler(|| Response::ok_body("hello world", "text/plain")),
				),
				endpoint_with_path(
					PathFilter::new("foo"),
					HttpMethod::Get,
					handler(|| Response::ok_body("hello foo", "text/plain")),
				),
			]));
		})
		.run();
}
