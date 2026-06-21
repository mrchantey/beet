//! Interactive terminal chat: a `.bsx` roster reduced into a thread, rendered
//! and driven through the agnostic charcell UI.
use beet::prelude::*;

/// The author scene: roster + system seed, reduced at runtime.
const SCENE: &str = include_str!("chat.bsx");

fn main() {
	env_ext::load_dotenv();
	App::new()
		.add_plugins((
			MinimalPlugins,
			ThreadPlugin::default(),
			ThreadUiPlugin,
			CharcellTuiPlugin,
		))
		.add_systems(Startup, setup)
		.run();
}

/// Reduce the `.bsx` scene into a window + behavior, then mount an interactive
/// charcell chat over it; the composer drives the agent's replies, no stdin.
fn setup(world: &mut World) -> Result {
	let thread = BsxTemplate::parse_entry(world, SCENE)?.spawn(world)?;
	reduce_threads_now(world);
	world.spawn(thread_chat_tui(thread));
	Ok(())
}
