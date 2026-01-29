//! A minimal server example using ExchangeSpawner
use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			ServerPlugin::default(),
		))
		.add_systems(Startup, |mut commands: Commands| {
			commands.spawn((
				HttpServer::default(),
				handler_exchange(|_, _| {
					Response::ok_body("hello world", "text/plain")
				}),
			));
		})
		.run();
}
