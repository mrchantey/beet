//! Example of using the beet router
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
				endpoint(HttpMethod::Get, || Response::ok_body(
					"hello world",
					"text/plain"
				),),
				endpoint_with_path("foo", HttpMethod::Get, || {
					Response::ok_body("hello foo", "text/plain")
				},),
			]));
		})
		.run();
}
