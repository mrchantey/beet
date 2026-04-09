use beet::prelude::*;
use beet_infra::bindings::AwsS3BucketDetails;


fn main() {
	App::new()
		.add_plugins((MinimalPlugins, InfraPlugin, LogPlugin {
			// level: Level::TRACE,
			..default()
		}))
		.add_systems(Startup, setup)
		.run();
}


fn setup(mut commands: Commands) {
	commands
		.spawn((
			Stack::new("lambda-example").with_backend(LocalBackend::default()),
			BucketBlock::new("my-bucket", AwsS3BucketDetails::default()),
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
			// project.apply().await?;
			// project.destroy().await?;
			Ok(())
		});
}
