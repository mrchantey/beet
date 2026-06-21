//! Persisting chat to the filesystem: a pinned-id `.bsx` roster whose
//! conversation is adopted by seed hash across reloads, driven by the agnostic
//! charcell composer.
//!
//! ```sh
//! cargo run --example persistent_chat --features=thread_ui,template_serde
//! cargo run --example persistent_chat --features=thread_ui,template_serde -- --new
//! ```
use beet::prelude::*;

/// Directory the thread's records are stored under, one subdir per record type.
const STORE_DIR: &str = "examples/thread/persistent_chat";

/// The author scene: a persisted roster with pinned actor ids (the seed hash and
/// `ActorRef` bindings depend on them being stable across reloads).
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
		// spawn the `.bsx` scene with its store, then adopt the matching stored
		// thread by seed hash (or bootstrap). No turn trigger: the composer and
		// `reply_to_user_posts` drive the conversation event-driven.
		let store_component = store.clone();
		let thread = world
			.with(move |world: &mut World| -> Result<Entity> {
				let entity =
					BsxTemplate::parse_entry(world, SCENE)?.spawn(world)?;
				world.entity_mut(entity).insert(store_component);
				Ok(entity)
			})
			.await?;
		adopt_thread(world.clone(), store, thread).await?;
		world
			.with(move |world: &mut World| {
				world.spawn(thread_chat_tui(thread));
			})
			.await;
		Ok(())
	});
}
