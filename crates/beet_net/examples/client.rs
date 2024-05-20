use beet_net::prelude::*;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;


#[derive(Debug, Default, Component, Serialize, Deserialize)]
struct MyComponent(pub i32);

#[tokio::main]
async fn main() {
	// pretty_env_logger::init();

	tokio::spawn(spawn_sender());
	App::new()
		.add_plugins((
			LogPlugin::default(),
			MinimalPlugins,
			ReplicatePlugin,
			ReplicateComponentPlugin::<MyComponent>::default(),
			NativeClientPlugin::default(),
		))
		.add_systems(Update, update)
		.run();
}


async fn spawn_sender() {
	std::thread::sleep(std::time::Duration::from_secs(1));
	App::new()
		.add_plugins((
			// LogPlugin::default(),
			MinimalPlugins,
			ReplicatePlugin,
			ReplicateComponentPlugin::<MyComponent>::default(),
			NativeClientPlugin::default(),
		))
		.add_systems(Startup, start)
		.run();
}


fn start(mut commands: Commands) {
	commands.spawn((Replicate::default(), MyComponent(7)));
}

fn update(query: Query<(Entity, &MyComponent), Added<MyComponent>>) {
	// log::info!("update");
	for entity in query.iter() {
		println!("{:?}", entity);
	}
}
