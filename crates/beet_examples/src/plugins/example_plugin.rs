use crate::prelude::*;
#[cfg(any(target_arch = "wasm32", feature = "tokio"))]
use beet::prelude::*;
use bevy::asset::AssetMetaCheck;
use bevy::prelude::*;
use forky_bevy::systems::close_on_esc;

#[derive(Default)]
pub struct ExamplePlugin;


#[cfg(feature = "tokio")]
const DEFAULT_SOCKET_URL: &str = "ws://127.0.0.1:3000/ws";

impl Plugin for ExamplePlugin {
	fn build(&self, app: &mut App) {
		assert_local_assets();

		#[cfg(target_arch = "wasm32")]
		app.add_transport(WebEventClient::new_with_window());

		#[cfg(feature = "tokio")]
		app.add_transport(NativeWsClient::new(DEFAULT_SOCKET_URL).unwrap());

		app
			.add_plugins(ExampleReplicatePlugin)
			.add_plugins(
				DefaultPlugins
					.set(WindowPlugin {
						primary_window: Some(Window {
							fit_canvas_to_parent: true,
							canvas: canvas(),
							resizable: true,
							..default()
						}),
						..default()
					})
					.set(AssetPlugin {
						file_path: assets_path(),
						meta_check: AssetMetaCheck::Never,
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
	// return "/wasm/assets".into();
	// return "https://demo.beetmash.com/wasm/assets".into();
	return "https://storage.googleapis.com/beet-examples/assets".into();
	#[cfg(not(target_arch = "wasm32"))]
	return "assets".into();
}


fn assert_local_assets() {
	#[cfg(not(target_arch = "wasm32"))]
	{
		let path = std::path::Path::new("assets/README.md");
		if !path.exists() {
			panic!(
				r#"
🐝🐝🐝

Howdy! Beet examples use large assets that are stored remotely. 

Windows:

1. Download https://storage.googleapis.com/beet-misc/assets.tar.gz
2. Unzip into `./assets`

Linux/MacOS:

curl -o ./assets.tar.gz https://storage.googleapis.com/beet-misc/assets.tar.gz
tar -xzvf ./assets.tar.gz
rm ./assets.tar.gz

🐝🐝🐝
"#
			);
		}
	}
}
