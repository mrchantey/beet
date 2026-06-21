//! Mini coding agent that creates files in a local store. The roster, the
//! `<StoreToolset/>` and the store mount are all authored in `.bsx`; `main` just
//! loads the scene.
//!
//! Run with:
//! ```sh
//! cargo run --example coding_agent --features thread,tui,fs,router,rustls-tls
//! ```
use beet::prelude::*;

const SCENE: &str = include_str!("coding_agent.bsx");

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
