use beet_core::prelude::*;
use bevy::prelude::*;
use bevy::tasks::futures_lite::future;

#[rustfmt::skip]
fn main() {
	App::new()
		.add_plugins((MinimalPlugins, AsyncPlugin))
		.init_resource::<Count>()
		// .add_systems(Startup, my_func)
		.add_systems(Startup, (count_to_five, await_return))
		.run();
}



#[derive(Default, Resource)]
struct Count(usize);

#[async_system]
async fn await_return(world: &mut World) {
	let fut = world.run_system_cached(returns_count).unwrap();
	let count = fut.await;
	// println!("returned count: {}", count);
}

///
#[async_system]
async fn returns_count(mut count: ResMut<Count>) -> usize {
	count.0 += 1;
	future::yield_now().await;
	count.0
}
/// a system using futures and streams to count to five
#[async_system]
async fn count_to_five(mut count: ResMut<Count>) {
	count.0 += 1;
	println!("count: {}", count.0);
	future::yield_now().await;
	count.0 += 1;
	println!("future count: {}", count.0);
	while let _ = StreamCounter::new(3).await {
		count.0 += 1;
		println!("stream count: {}", count.0);
	}
}
