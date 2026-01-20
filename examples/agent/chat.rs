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
				CliServer,
				flow_exchange(|| {
					(InfallibleSequence, children![
						EndpointBuilder::new().with_handler(oneshot()),
						EndpointBuilder::new().with_path("repl").with_action(
							|| { Response::ok_body("repl", "text/plain") }
						)
					])
				}),
			));
		})
		.run();
}



