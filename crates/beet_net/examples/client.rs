use beet_net::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;


#[derive(Debug, Default, Component, Serialize, Deserialize)]
struct MyComponent(pub i32);

#[tokio::main]
async fn main() {
	// pretty_env_logger::init();
	App::new()
		.add_plugins((
			// LogPlugin::default(),
			MinimalPlugins,
			ReplicatePlugin,
			ReplicateTypePlugin::<MyComponent>::default(),
			NativeClientPlugin::default(),
		))
		.add_systems(Startup, start)
		.run();
}

fn start(mut commands: Commands) {
	commands.spawn((Replicate::default(), MyComponent(7)));
}

// fn update(query: Query<(Entity, &MyComponent), Added<MyComponent>>) {
// 	// log::info!("update");
// 	for (_entity, comp) in query.iter() {
// 		log::info!("SUCCESS - {:?}", comp);
// 	}
// }
