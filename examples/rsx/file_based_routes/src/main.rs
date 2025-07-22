use beet::prelude::*;
use file_based_routes::prelude::*;



fn main() -> Result {
	if !cfg!(feature = "config")
		&& !cfg!(feature = "server")
		&& !cfg!(feature = "client")
	{
		panic!("One feature must be enabled: config, server, client");
	}

	let mut app = App::new();
	#[cfg(feature = "config")]
	app.add_plugins(ConfigPlugin);

	app.run();

	Ok(())
}


#[cfg(feature = "server")]
fn server_plugin(app: &mut App) {
	app.world_mut()
		.spawn((children![pages_routes(), docs_routes(), actions_routes()]));
}
