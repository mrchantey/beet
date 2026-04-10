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
			level: Level::WARN,
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
				.await?;

			let bucket = entity
				.with_state::<StackQuery, _>(|entity, query| {
					query.s3_provider(entity)
				})
				.await?;

			// Reset state in case of backend change
			project.force_destroy().await;
			if bucket.bucket_exists().await.unwrap_or(false) {
				println!("🧹 Cleaning up stale bucket..");
				bucket.bucket_remove().await.ok();
			}

			println!("🔨 Validating..");
			project.validate().await?;


			println!("🔨 Planning..");
			let _plan = project.plan().await?;

			// state file and bucket dont exist yet, we are pre-apply
			println!(
				"📦 State file exists: {}",
				project.state_file().exists().await?
			);
			println!("🪣 Bucket Exists: {}", bucket.bucket_exists().await?);

			println!("🔨 Applying..");
			project.apply().await?;

			println!(
				"📦 State File exists: {}",
				project.state_file().exists().await?
			);
			println!("🪣 Bucket Exists: {}", bucket.bucket_exists().await?);

			let path = RelPath::new("foo.md");
			let content = "bar";

			println!(
				"📄 Bucket File Exists: {}",
				bucket.get(&path).await.is_ok()
			);

			println!("🔨 Inserting File..");
			bucket.insert(&path, content.into()).await?;
			let bytes = bucket.get(&path).await?;
			println!("📄 Bucket File Matches: {}", bytes == content.as_bytes());

			println!("🔨 Destroying..");
			project.destroy().await?;

			println!(
				"📦 State file exists: {}",
				project.state_file().exists().await?
			);
			println!("🪣 Bucket Exists: {}", bucket.bucket_exists().await?);

			entity.world().write_message(AppExit::Success);
			Ok(())
		});
}
