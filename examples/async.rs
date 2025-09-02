use beet_core::prelude::*;
use bevy::prelude::*;
use bevy::tasks::futures_lite::future;

#[rustfmt::skip]
fn main() {
	App::new()
		.add_plugins((MinimalPlugins, AsyncPlugin))
		.init_resource::<Count>()
		.add_systems(Startup, setup)
		.run();
}

#[async_system]
async fn setup(world: &mut World) {
	// await statements are moved so assign the future
	// to avoid moving &mut World
	let fut = world.run_system_cached(count_to_five).unwrap();
	let count = fut.await;
	assert_eq!(count, 5);
	std::process::exit(0);
}

#[derive(Default, Resource)]
struct Count(usize);

/// an async system using futures and streams to count to five
/// then returning the final value.
#[async_system]
async fn count_to_five(mut count: ResMut<Count>) -> usize {
	count.0 += 1;
	println!("count: {}", count.0);
	future::yield_now().await;
	count.0 += 1;
	println!("count: {}", count.0);
	while let _ = StreamCounter::new(3).await {
		count.0 += 1;
		println!("count: {}", count.0);
	}
	count.0
}
