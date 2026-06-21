//! Interactive terminal chat: a `.bsx` author scene reduced into a thread,
//! rendered and driven through the agnostic charcell UI.
//!
//! The scene is the whole program: a `Repeat[Sequence[agent, user]]` whose user
//! turn (`{UserInput}`) waits for the composer, kicked by `{RunThread}`. `main`
//! just loads it, so there is no setup-system glue.
use beet::prelude::*;

const SCENE: &str = include_str!("chat.bsx");

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
