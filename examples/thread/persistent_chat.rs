//! Persisting chat to the filesystem: a pinned-id `.bsx` author scene whose
//! conversation is adopted by seed hash across reloads, driven by the agnostic
//! charcell composer.
//!
//! ```sh
//! cargo run --example persistent_chat --features thread,tui,template_serde,rustls-tls
//! cargo run --example persistent_chat --features thread,tui,template_serde,rustls-tls -- --new
//! ```
use beet::prelude::*;

/// Directory the thread's records are stored under, one subdir per record type.
const STORE_DIR: &str = "examples/thread/persistent_chat";

/// The author scene: a `Repeat[Sequence[agent, user]]` with pinned actor ids (the
/// seed hash and `ActorRef` bindings depend on them being stable across reloads).
const SCENE: &str = include_str!("persistent_chat.bsx");

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

/// Spawn the scene, mount the store on the `Thread` (nested under the loop), adopt
/// the stored thread by seed hash (or bootstrap), then kick the loop and render
/// the chat. The adoption must finish before the kick, so this stays in Rust
/// rather than the markup `{RunThread}` kick the auto examples use.
fn setup(async_commands: AsyncCommands) {
	cfg_if! {
		if #[cfg(feature="aws_sdk")]{
			// swap out for s3 storage by changing the store
			// see infra examples for configuring stores
			let blob = S3Store::new("some-bucket","some-region");
		}else{
			let blob = FsStore::new(WsPathBuf::default());
		}
	}

	// the thread's records live under a stable store path; the conversation
	// persists while the authored program is re-spawned each load
	let store = ThreadStore::new(BlobThreadStore::new(
		BlobStore::new(blob).with_subdir(SmolPath::new(STORE_DIR)),
	));
	let new_thread = CliArgs::parse_env().params.contains_key("new");

	async_commands.run(async move |world: AsyncWorld| -> Result {
		if new_thread {
			store.store_remove().await.ok();
		}
		// spawn + reduce, mounting the store on the Thread entity (nested under
		// the loop) where the persistence sync reads it
		let store_component = store.clone();
		let (root, thread) = world
			.with(move |world: &mut World| -> Result<(Entity, Entity)> {
				let root = BsxTemplate::parse_entry(world, SCENE)?.spawn(world)?;
				ThreadWindow::reduce_now(world);
				let thread = world
					.query_filtered::<Entity, With<Thread>>()
					.iter(world)
					.next()
					.ok_or_else(|| bevyhow!("no Thread in scene"))?;
				world.entity_mut(thread).insert(store_component);
				Ok((root, thread))
			})
			.await?;
		// adopt by seed hash before kicking, so a turn never runs on a stale
		// window; then kick the loop root and render the chat over the thread
		adopt_thread(world.clone(), store, thread).await?;
		world
			.with(move |world: &mut World| {
				world
					.entity_mut(root)
					.insert(CallOnSpawn::<(), Outcome>::new(()));
				world.spawn(thread_chat_tui(thread));
			})
			.await;
		Ok(())
	});
}
