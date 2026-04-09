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
			let plan = project.plan().await?;
			println!("{plan}");
			// hey agent leave this for now do not attempt to implement
			// ...
			process_ext::exit(0);
			Ok(())
		});
}
