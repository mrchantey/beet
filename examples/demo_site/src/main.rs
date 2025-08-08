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
	app.world_mut().spawn(collections());
}

#[cfg(feature = "server")]
fn server_plugin(app: &mut App) {
	// create a router, specifying the plugin for Router Apps
	app.insert_resource(Router::new_bundle(routes));
}

#[cfg(feature = "client")]
fn client_plugin(app: &mut App) {
	// clients load a scene file from the html,
	// so we need to register any type with a client load directive
	app
		.register_type::<ClientIslandRoot<ClientCounter>>()
		.register_type::<ClientIslandRoot<ServerCounter>>();
}
