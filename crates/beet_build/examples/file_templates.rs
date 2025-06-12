use beet_build::as_beet::*;
use bevy::prelude::*;
use beet_bevy::prelude::*;


fn main() {
	let mut app = App::new();
	app.add_plugins((
		BeetConfig::default(),
		NodeTokensPlugin,
		BuildTemplatesPlugin,
	));

	app.update();

	let scene = app.build_scene();
	println!("Exported Scene: {}", scene);
}
