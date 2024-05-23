use crate::assets_path;
use bevy::prelude::*;



#[derive(Default)]
pub struct ExamplePlugin;


impl Plugin for ExamplePlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						fit_canvas_to_parent: true,
						canvas: Some("#beet-canvas".into()),
						..default()
					}),
					..default()
				})
				.set(AssetPlugin {
					file_path: assets_path(),
					..default()
				})
				.build(),
		);

		// #[cfg(target_arch = "wasm32")]
		// app.add_systems(Startup, wasm_funcs::set_canvas_ready);
	}
}

