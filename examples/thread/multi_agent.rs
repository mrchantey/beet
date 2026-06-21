//! Two agents converse: a `.bsx` roster reduced into a thread, looped by
//! `Repeat` and rendered through the agnostic charcell UI as a transcript.
//!
//! The scene is the whole program: a `Repeat[Sequence[agent, agent]]` kicked by
//! `{RunThread}`. `main` just loads it, so there is no setup-system glue.
use beet::prelude::*;

const SCENE: &str = include_str!("multi_agent.bsx");

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
