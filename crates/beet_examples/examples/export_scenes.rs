use beet::prelude::*;
use beet_examples::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;


#[path = "./basics/scenes/mod.rs"]
mod basics_scenes;


struct SceneItem {
	pub app: &'static str,
	pub name: &'static str,
	pub system: SystemConfigs,
}


impl SceneItem {
	pub fn save(self) {
		let mut app = App::new();
		app.add_plugins((
			TaskPoolPlugin::default(),
			DefaultBeetPlugins::default(),
			// ExamplePlugin::default(),
		))
		.finish();
		Schedule::default()
			.add_systems(self.system)
			.run(app.world_mut());

		let filename = format!(
			"target/scenes/{}/{}.ron",
			// "crates/beet_examples/examples/{}/scenes/{}.ron",
			self.app,
			self.name
		);
		Schedule::default()
			.add_systems(save_scene(&filename))
			.run(app.world_mut());
		// schedu
	}
}

fn main() {
	let scenes = vec![
		SceneItem {
			app: "basics",
			name: "hello_world",
			system: basics_scenes::hello_world.into_configs(),
		},
		SceneItem {
			app: "basics",
			name: "hello_net",
			system: basics_scenes::hello_net.into_configs(),
		},
	];
	for scene in scenes {
		scene.save();
	}
}
