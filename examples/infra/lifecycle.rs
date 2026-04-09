//! Demonstrates the full lifecycle of an infra project
//!
//!
//!
use beet::prelude::*;


fn main() {
	App::new()
		.add_plugins((MinimalPlugins, InfraPlugin, LogPlugin {
			level: Level::TRACE,
			..default()
		}))
		.add_systems(Startup, setup)
		.run();
}


fn setup(mut commands: Commands) {
	cfg_if! {
		if #[cfg(feature="aws")] {
			let backend = S3Backend::default();
		} else {
			let backend = LocalBackend::default();
		}
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

			println!("Validating..");
			project.validate().await?;
			println!("Planning..");
			let plan = project.plan().await?;
			println!("Plan generated: \n{plan}");
			project.apply().await?;
			let path = RoutePath::new("foo.md");
			let content = "bar";
			let provider = entity
				.with_state::<StackQuery, _>(|entity, query| {
					query.s3_provider(entity)
				})
				.await?;
			println!("Applying..");
			// it should not not yet be set
			provider.get(&path).await.unwrap_err();
			println!("Inserting..");
			provider.insert(&path, content.into()).await?;
			let bytes = provider.get(&path).await?;
			assert_eq!(bytes, content.as_bytes());

			println!("Destroying..");
			project.destroy().await?;

			entity.world().write_message(AppExit::Success);
			Ok(())
		});
}
