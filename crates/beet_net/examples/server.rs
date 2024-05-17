use beet_net::prelude::*;
// use bevy::MinimalPlugins;
use bevy::DefaultPlugins;


fn main() {
	let mut apps = net_apps();
	apps.add_plugins(DefaultPlugins, DefaultPlugins, ());
	apps.for_each(|a| a.finish());
	loop {
		apps.for_each(|a| a.update());
		std::thread::sleep(std::time::Duration::from_secs(1));
	}
	// .run();
}
