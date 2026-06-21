//! Mini coding agent that creates files in a local store. The roster + standard
//! `<StoreToolset/>` are authored in `.bsx`; only the store and run glue are Rust.
//!
//! Run with:
//! ```sh
//! cargo run --example coding_agent --features thread_ui,fs,router
//! ```
use beet::prelude::*;

/// The author scene: an agent equipped with the store toolset, looped while it
/// keeps calling tools.
const SCENE: &str = include_str!("coding_agent.bsx");

#[beet::main]
async fn main() {
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

fn setup(async_commands: AsyncCommands) {
	async_commands.run(async move |world: AsyncWorld| -> Result {
		let store = FsStore::new(
			AbsPathBuf::new_workspace_rel(".beet/coding_agent").unwrap(),
		);
		// mount the store on the behavior root, reduce the scene, render the
		// transcript, then run the tool loop to completion
		let root = world
			.with(move |world: &mut World| -> Result<Entity> {
				let root =
					BsxTemplate::parse_entry(world, SCENE)?.spawn(world)?;
				world.entity_mut(root).insert(store);
				reduce_threads_now(world);
				let thread = world
					.query_filtered::<Entity, With<Thread>>()
					.iter(world)
					.next()
					.ok_or_else(|| bevyhow!("no Thread in scene"))?;
				world.spawn(thread_tui(thread));
				Ok(root)
			})
			.await?;
		world.entity(root).call::<(), Outcome>(()).await?;
		world.write_message(AppExit::Success).await;
		Ok(())
	});
}
