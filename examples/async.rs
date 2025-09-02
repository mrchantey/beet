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
	println!("return count: {}", count);
	assert_eq!(count, 5);
}

#[derive(Default, Resource)]
struct Count(usize);

/// an async system that returns a value
// #[async_system]
// async fn returns_count(mut count: ResMut<Count>) -> usize {
// 	count.0 += 1;
// 	future::yield_now().await;
// 	count.0
// }

/// an async system using futures and streams to count to five
#[async_system]
async fn count_to_five(mut count: ResMut<Count>) -> usize {
	future::yield_now().await;
	count.0 += 1;
	println!("future count: {}", count.0);
	while let _ = StreamCounter::new(3).await {
		count.0 += 1;
		println!("stream count: {}", count.0);
	}
	println!("after count: {}", count.0);
	count.0
}
