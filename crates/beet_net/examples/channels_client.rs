use beet_net::prelude::*;
use bevy::app::App;
// use bevy::MinimalPlugins;
use bevy::DefaultPlugins;
use lightyear::client::plugin::ClientPlugins;
use lightyear_common::apps::default_settings;
use lightyear_common::apps::settings_to_client_config_crossbeam;
use std::time::Duration;


fn main() {
	let (send, recv) = crossbeam_channel::unbounded();

	let config = settings_to_client_config_crossbeam(
		default_settings(),
		send.clone(),
		recv,
		None,
	);


	std::thread::spawn(move || {
		// std::thread::sleep(Duration::from_secs(1));
		// send.send(vec![0, 1, 2, 3, 4]).unwrap();
		// std::thread::sleep(Duration::from_secs(1));
		// send.send(vec![0, 1, 2, 3, 4]).unwrap();
		// std::thread::sleep(Duration::from_secs(1));
		// send.send(vec![0, 1, 2, 3, 4]).unwrap();
	});

	let mut app = App::new();
	app.add_plugins((
		ClientPlugins { config },
		BaseClientPlugin,
		BaseSharedPlugin,
		DefaultPlugins,
	))
	.finish();

	loop {
		app.update();
		std::thread::sleep(Duration::from_millis(16));
	}

	// apps.add_plugins(, DefaultPlugins, ());
	// apps.for_each(|a| a.finish());
	// loop {
	// 	apps.for_each(|a| a.update());
	// 	std::thread::sleep(std::time::Duration::from_secs(1));
	// }
	// .run();
}
