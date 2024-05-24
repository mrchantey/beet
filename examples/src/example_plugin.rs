use bevy::prelude::*;
use forky_bevy::systems::close_on_esc;



#[derive(Default)]
pub struct ExamplePlugin;


impl Plugin for ExamplePlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						fit_canvas_to_parent: true,
						canvas: canvas(),
						..default()
					}),
					..default()
				})
				.set(AssetPlugin {
					file_path: assets_path(),
					..default()
				})
				.build(),
		)
		.add_systems(Update, close_on_esc);
	}
}


fn canvas() -> Option<String> {
	// #[cfg(debug_assertions)]
	return Some("#beet-canvas".into());
	// #[cfg(not(debug_assertions))]
	// return None;
}


fn assets_path() -> String {
	#[cfg(target_arch = "wasm32")]
	return "https://storage.googleapis.com/beet-examples/assets".into();
	// return "../assets".into();
	#[cfg(not(target_arch = "wasm32"))]
	return "assets".into();
}
