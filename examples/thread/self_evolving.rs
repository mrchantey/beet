//! A self-evolving agent whose only memory is a blob store it reads and writes
//! across turns. The roster + standard `<StoreToolset/>` are authored in `.bsx`;
//! only the store and run glue stay in Rust.
use beet::prelude::*;

/// The author scene: a ten-turn loop over an agent equipped with the store toolset.
const SCENE: &str = include_str!("self_evolving.bsx");

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
		let store_path = WsPathBuf::new("target/examples/self_evolving");
		fs_ext::remove(&store_path).ok();
		let store = FsStore::new(store_path);

		// mount the store on the behavior root (an ancestor of the toolset), reduce
		// the scene, and render the transcript before the loop runs
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
