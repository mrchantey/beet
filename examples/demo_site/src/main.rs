#![allow(unused)]
use beet::prelude::*;
use demo_site::prelude::*;



fn main() -> Result {
	#[cfg(not(any(
		feature = "launch",
		feature = "server",
		feature = "client"
	)))]
	panic!("one of 'launch', 'server', or 'client' features must be enabled");

	let mut app = App::new();

	#[cfg(feature = "launch")]
	app.add_plugins(launch_plugin);
	#[cfg(feature = "server")]
	app.add_plugins(server_plugin);
	#[cfg(feature = "client")]
	app.add_plugins(client_plugin);

	app.run();

	Ok(())
}


#[cfg(feature = "server")]
#[rustfmt::skip]
fn server_plugin(app: &mut App) {
	app
		.add_plugins(RouterPlugin)
		.world_mut().spawn((
			Name::new("File Based Routes"),
			children![
				pages_routes(), 
				docs_routes(), 
				actions_routes()
			],
			// this is placed last to ensure it runs after the routes
			RouteHandler::layer(||{
				let mut state = AppState::get();
				state.num_requests += 1;
				AppState::set(state);
			
			})
	));
}


#[cfg(feature = "client")]
fn client_plugin(app: &mut App) {
	app.add_plugins(TemplatePlugin)
		.insert_resource(TemplateFlags::None)
		.register_type::<ClientIslandRoot<ClientCounter>>()
		.register_type::<ClientIslandRoot<ServerCounter>>()
		.set_runner(ReactiveApp::runner);
}
