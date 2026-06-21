//! Two agents converse: a `.bsx` roster reduced into a thread, looped by
//! `Repeat` and rendered through the agnostic charcell UI.
use beet::prelude::*;

/// The author scene: a `Repeat` over a two-agent `Sequence`.
const SCENE: &str = include_str!("multi_agent.bsx");

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

/// Reduce the scene, kick the endless exchange with `CallOnSpawn`, and render
/// the transcript. The `Repeat` root drives the loop; no user input.
fn setup(world: &mut World) -> Result {
	let root = BsxTemplate::parse_entry(world, SCENE)?.spawn(world)?;
	world
		.entity_mut(root)
		.insert(CallOnSpawn::<(), Outcome>::new(()));
	reduce_threads_now(world);
	let thread = world
		.query_filtered::<Entity, With<Thread>>()
		.iter(world)
		.next()
		.ok_or_else(|| bevyhow!("no Thread in scene"))?;
	world.spawn(thread_tui(thread));
	Ok(())
}
