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
	commands
		.spawn((
			Stack::new("bucket-example").with_backend(LocalBackend::default()),
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
			project.plan().await?;
			// hey agent leave this for now do not attempt to implement
			// let provider = entity
			// 	.with_state::<StackQuery, _>(|entity, query| {
			// 		query.s3_provider(entity)
			// 	})
			// 	.await?;
			// println!("Applying..");
			// project.apply().await?;
			// let path = RoutePath::new("foo.md");
			// let content = "bar";
			// // not yet set
			// provider.get(&path).await.unwrap_err();
			// println!("Inserting..");
			// provider.insert(&path, content.into()).await?;
			// let bytes = provider.get(&path).await?;
			// assert_eq!(bytes, content.as_bytes());

			// println!("Destroying..");
			// project.destroy().await?;

			Ok(())
		});
}
