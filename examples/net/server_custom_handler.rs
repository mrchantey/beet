//! The default handler spawns an entity with the [`Request`]
//! and returns the [`response`] as soon as its inserted.
//! This can be overridden by setting a custom handler,
//! for a more detailed example of a custom handler see [`FlowRouterPlugin`]
use beet::prelude::*;
use bevy::log::LogPlugin;

fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			ServerPlugin::with_server(
				Server::default().with_handler(my_handler),
			),
		))
		.run();
}

async fn my_handler(_server: AsyncEntity, _request: Request) -> Response {
	Response::ok_body("hello custom", "text/plain")
}
