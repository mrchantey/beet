//! Demonstrates the full lifecycle of an infra project.
//!
//! Handles cleanup of stale AWS resources from previously interrupted runs
//! before applying infrastructure changes.
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
	let backend: StackBackend = if args.params.contains_key("aws") {
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

			let provider = entity
				.with_state::<StackQuery, _>(|entity, query| {
					query.s3_provider(entity)
				})
				.await?;

			// Reset state in case of a previously interrupted run.
			// force_destroy only cleans terraform state; it cannot remove
			// AWS resources whose state was already lost. Explicitly
			// remove the managed bucket if it still exists.
			project.force_destroy().await;
			if provider.bucket_exists().await.unwrap_or(false) {
				println!("🧹 Cleaning up stale bucket..");
				provider.bucket_remove().await.ok();
			}

			println!("🔨 Validating..");
			project.validate().await?;

			println!("🔨 Planning..");
			let plan = project.plan().await?;
			println!("🧭 Plan generated: \n{plan}");

			println!("🪣 Bucket Exists: {}", provider.bucket_exists().await?);

			println!("🔨 Applying..");
			project.apply().await?;

			println!("🪣 Bucket Exists: {}", provider.bucket_exists().await?);

			let path = RelPath::new("foo.md");
			let content = "bar";

			println!("📄 File Exists: {}", provider.get(&path).await.is_ok());

			println!("🔨 Inserting File..");
			provider.insert(&path, content.into()).await?;
			let bytes = provider.get(&path).await?;
			println!("📄 File Matches: {}", bytes == content.as_bytes());

			println!("🔨 Destroying..");
			project.destroy().await?;

			println!("🪣 Bucket Exists: {}", provider.bucket_exists().await?);

			entity.world().write_message(AppExit::Success);
			Ok(())
		});
}
