//! A basic example of using the beet router
use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			RouterPlugin::default(),
		))
		.add_systems(Startup, |mut commands: Commands| {
			commands.spawn((
				HttpServer::new(5000),
				flow_exchange(|| {
					// InfallibleSequence ensures all endpoints are checked
					// even if the previous one did not match
					(InfallibleSequence, children![
						EndpointBuilder::get().with_action(|| {
							Response::ok_body("hello world", "text/plain")
						}),
						EndpointBuilder::get().with_path("foo").with_action(
							|| {
								Response::ok_body(
									"<div>hello foo</div>",
									"text/html",
								)
							},
						),
					])
				}),
			));
		})
		.run();
}
