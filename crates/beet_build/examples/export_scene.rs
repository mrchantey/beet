use beet_bevy::prelude::*;
use beet_build::as_beet::*;
use bevy::prelude::*;


fn main() {
	let mut app = App::new();
	app.add_plugins((ParseRsxTokensPlugin, StaticScenePlugin));
	app.world_mut().spawn(rsx! {
		<div>
			<h1>Export Scene Example</h1>
			<style>
				h1 {
					color: blue;
				}
			</style>
		</div>
	});
	app.update();

	let scene = app.build_scene();
	println!("Exported Scene: {}", scene);
}
