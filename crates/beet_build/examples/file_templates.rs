use beet_bevy::prelude::*;
use beet_build::as_beet::*;
use bevy::prelude::*;
use sweet::bevy::CoreAppExtSweet;


fn main() {
	let scene = App::new()
		.add_plugins((NodeTokensPlugin, StaticScenePlugin))
		.init_resource::<StaticSceneConfig>()
		.update_then()
		.build_scene();

	println!("Exported Scene: {}", scene);
}
