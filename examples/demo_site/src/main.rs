#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused)]
use beet::prelude::*;
use demo_site::prelude::*;

fn main() {
	App::new()
		.add_plugins((
			BeetPlugins,
			#[cfg(feature = "launch")]
			launch_plugin,
			#[cfg(feature = "server")]
			server_plugin,
			#[cfg(feature = "client")]
			client_plugin,
		))
		.run();
}

#[cfg(feature = "launch")]
fn launch_plugin(app: &mut App) {
	app.world_mut().spawn((
		RouteCodegenRoot::default(),
		children![
			pages_collection(), 
			docs_collection(), 
			actions_collection()
		],
	));
}

#[cfg(feature = "server")]
fn server_plugin(app: &mut App) {
	app.world_mut().spawn((
		children![
			pages_routes(), 
			docs_routes(), 
			actions_routes()
		],
		// this is placed last to ensure it runs after all handlers
		RouteHandler::layer(|| {
			let mut state = AppState::get();
			state.num_requests += 1;
			AppState::set(state);
		}),
	));
}

#[cfg(feature = "client")]
fn client_plugin(app: &mut App) {
	app
		.register_type::<ClientIslandRoot<ClientCounter>>()
		.register_type::<ClientIslandRoot<ServerCounter>>();
}
