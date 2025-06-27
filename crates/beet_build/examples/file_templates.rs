use beet_bevy::prelude::*;
use beet_build::as_beet::*;
use bevy::prelude::*;


fn main() {
	let mut app = App::new();
	app.add_plugins((NodeTokensPlugin, StaticScenePlugin))
		.world_mut()
		.spawn(BuildFileTemplates::default());

	app.update();

	let scene = app.build_scene();
	println!("Exported Scene: {}", scene);
}
