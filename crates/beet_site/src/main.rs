#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused)]
use beet::prelude::*;
use beet_site::prelude::*;

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

	let mut config = WorkspaceConfig::default();
	config.filter
		.include("*/crates/beet_design/src/**/*")
		.include("*/crates/beet_site/src/**/*");

	app.insert_resource(config);

	app.world_mut().spawn(collections());
}

#[cfg(feature = "server")]
fn server_plugin(app: &mut App) {
	app.insert_resource(Router::new(|app:&mut App|{app.world_mut().spawn(routes());}));
}

#[cfg(feature = "client")]
fn client_plugin(app: &mut App) {
	app
		.register_type::<ClientIslandRoot<beet_design::mockups::route3::Inner>>()
		.register_type::<ClientIslandRoot<beet_design::mockups::route4::Inner>>()
		.register_type::<ClientIslandRoot<ClientCounter>>()
		.register_type::<ClientIslandRoot<ServerCounter>>();
}