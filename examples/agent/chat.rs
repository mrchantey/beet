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
				ExchangeSpawner::new_flow(|| {
					(InfallibleSequence, children![
						EndpointBuilder::new(|| {
							todo!("agent");
							Response::ok_body("hello world", "text/plain")
						}),
					])
				}),
			));
		})
		.run();
}
