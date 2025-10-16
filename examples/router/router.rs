//! A basic example of using the beet router
use beet::prelude::*;

// boo tokio todo replace reqwest
#[tokio::main]
async fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			RouterPlugin::default(),
		))
		.add_systems(Startup, |mut commands: Commands| {
			// The RouteServer is a beet_flow pattern, triggering `GetOutcome`
			// and returning the Response once `Outcome` is triggered
			commands.spawn((
				RouteServer,
				// this sequence type will ensure all endpoints are checked
				// even if the previous one did not match
				InfallibleSequence,
				children![
					EndpointBuilder::get().with_handler(|| Response::ok_body(
						"hello world",
						"text/plain"
					)),
					EndpointBuilder::get().with_path("foo").with_handler(
						|| { Response::ok_body("hello foo", "text/plain") },
					),
				],
			));
		})
		.run();
}
