//! Demonstrates the full lifecycle of an infra project,
//! by deploying a stack with a single s3 bucket, then tearing it down.
//!
//! logs are outputted to show the time at which various
//! parts are created and destroyed:
//!
//! ```sh
//! # local backend
//! cargo run --example lifecycle --features=bindings_aws_common,aws
//! # s3 backend
//! cargo run --example lifecycle --features=bindings_aws_common,aws -- --s3-backend
//! ```
use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((MinimalPlugins, InfraPlugin, LogPlugin {
			level: Level::INFO,
			..default()
		}))
		.add_systems(Startup, setup)
		.run();
}

fn setup(mut commands: Commands) {
	let args = CliArgs::parse_env();
	let backend: StackBackend = if args.params.contains_key("s3-backend") {
		S3Backend::default().into()
	} else {
		LocalBackend::default().into()
	};

	commands
		.spawn((
			Stack::new("bucket-example").with_backend(backend),
			S3BucketBlock::new("my-bucket"),
		))
		.queue_async(async move |entity| {
			let project = entity
				.with_state::<StackQuery, _>(|entity, query| {
					query.build_project(entity)
				})
				.await??;

			let store = entity
				.with_state::<StackQuery, _>(|entity, query| {
					query.store(entity).cloned()
				})
				.await??;

			// Reset state in case of backend change
			project.force_destroy().await;
			if store.store_exists().await.unwrap_or(false) {
				info!("🧹 Cleaning up stale store..");
				store.store_remove().await.ok();
			}

			info!("🔨 Validating..");
			project.validate().await?;

			info!("🔨 Planning..");
			let _plan = project.plan().await?;

			// state file and bucket dont exist yet, we are pre-apply
			info!(
				"📦 State file exists: {}",
				project.state_file().exists().await?
			);
			info!("🪣 BlobStore Exists: {}", store.store_exists().await?);

			info!("🔨 Applying..");
			project.apply().await?;

			info!(
				"📦 State File exists: {}",
				project.state_file().exists().await?
			);
			info!("🪣 BlobStore Exists: {}", store.store_exists().await?);

			let path = SmolPath::new("foo.md");
			let content = "bar";

			info!(
				"📄 BlobStore File Exists: {}",
				store.get(&path).await.is_ok()
			);

			info!("🔨 Inserting File..");
			store.insert(&path, content).await?;
			let bytes = store.get(&path).await?;
			info!("📄 BlobStore File Matches: {}", bytes == content.as_bytes());

			info!("🔨 Destroying..");
			project.destroy().await?;

			info!(
				"📦 State file exists: {}",
				project.state_file().exists().await?
			);
			info!("🪣 BlobStore Exists: {}", store.store_exists().await?);

			entity.world().write_message(AppExit::Success).await;
			Ok(())
		});
}
