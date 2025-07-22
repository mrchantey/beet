use beet::prelude::*;
use file_based_routes::prelude::*;



fn main() -> Result {
	#[cfg(not(any(
		feature = "config",
		feature = "server",
		feature = "client"
	)))]
	panic!("one of 'config', 'server', or 'client' features must be enabled");

	let mut app = App::new();

	#[cfg(feature = "config")]
	app.add_plugins(config_plugin);
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
		.add_plugins(AppRouterPlugin)
		.world_mut().spawn((
			Name::new("File Based Routes"),
			children![
				pages_routes(), 
				docs_routes(), 
				actions_routes()
			],
	));
}


#[cfg(feature = "client")]
fn client_plugin(app: &mut App) {
	app.add_plugins(TemplatePlugin)
		.insert_resource(TemplateFlags::None)
		.register_type::<ClientIslandRoot<ClientCounter>>()
		.set_runner(ReactiveApp::runner);
}
