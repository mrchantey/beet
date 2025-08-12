#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused)]
use beet::prelude::*;
use todo_app::prelude::*;

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
		.insert_resource(PackageConfig{
			title: "Todo App".to_string(),
			..pkg_config!()
		})
		.run();
}

#[cfg(feature = "client")]
fn client_plugin(app: &mut App) {
	// clients load a scene file from the html,
	// so we need to register any type with a client load directive
	app
		.register_type::<ClientIslandRoot<ClientCounter>>()
		.register_type::<ClientIslandRoot<ServerCounter>>();
}
