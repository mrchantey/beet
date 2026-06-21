//! A self-evolving agent whose only memory is a blob store it reads and writes
//! across turns. The roster, the `<StoreToolset/>` and the store mount are all
//! authored in `.bsx`; `main` just loads the scene.
use beet::prelude::*;

const SCENE: &str = include_str!("self_evolving.bsx");

fn main() {
	env_ext::load_dotenv();
	App::new()
		.add_plugins((
			MinimalPlugins,
			ThreadPlugin::default(),
			ThreadUiPlugin,
			CharcellTuiPlugin,
			ThreadScenePlugin::new(SCENE),
		))
		.run();
}
