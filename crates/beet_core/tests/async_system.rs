#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_core::as_beet::*;
use bevy::prelude::*;
use futures_lite::future;
use sweet::prelude::*;


#[derive(Resource)]
struct Count(usize);

#[async_system]
async fn exclusive_async_system(world: &mut World) {
	let _ = future::yield_now().await;
}
#[async_system]
async fn returns_stuff(world: &mut World) -> usize {
	let _ = future::yield_now().await;
	3
}

#[test]
fn futures() {
	let mut app = App::new();
	app.insert_resource(Count(0))
		.add_plugins((MinimalPlugins, AsyncPlugin));

	#[async_system]
	async fn my_system(mut count: ResMut<Count>) {
		let _ = future::yield_now().await;
		assert_eq!(count.0, 0);
		count.0 += 1;
		let _ = future::yield_now().await;
		{
			let _ = future::yield_now().await;
		}
		assert_eq!(count.0, 1);
		count.0 += 1;
		let _ = future::yield_now().await;
		assert_eq!(count.0, 2);
		count.0 += 1;
	}

	app.world_mut().run_system_cached(my_system).ok();
	app.world_mut()
		.query_once::<&AsyncStreamTask>()
		.iter()
		.count()
		.xpect()
		.to_be(1);
	app.update();
	app.update();
	app.update();
	app.update();
	app.world_mut()
		.query_once::<&AsyncStreamTask>()
		.iter()
		.count()
		.xpect()
		.to_be(0);
	app.world_mut().resource::<Count>().0.xpect().to_be(3);
}
#[test]
fn observers() {
	let mut app = App::new();
	app.insert_resource(Count(0))
		.add_plugins((MinimalPlugins, AsyncPlugin));

	#[derive(Event)]
	struct MyEvent;
	// compiles
	#[async_system]
	async fn my_exclusive_observer(_: Trigger<MyEvent>, world: &mut World) {
		let _ = future::yield_now().await;
		assert_eq!(world.resource::<Count>().0, 0);
		world.resource_mut::<Count>().0 += 1;
		let _ = future::yield_now().await;
		{
			let _ = future::yield_now().await;
		}
		assert_eq!(world.resource::<Count>().0, 1);
		world.resource_mut::<Count>().0 += 1;
		let _ = future::yield_now().await;
		assert_eq!(world.resource::<Count>().0, 2);
		world.resource_mut::<Count>().0 += 1;
	}

	#[async_system]
	async fn my_observer(_: Trigger<MyEvent>, mut count: ResMut<Count>) {
		let _ = future::yield_now().await;
		assert_eq!(count.0, 0);
		count.0 += 1;
		let _ = future::yield_now().await;
		{
			let _ = future::yield_now().await;
		}
		assert_eq!(count.0, 1);
		count.0 += 1;
		let _ = future::yield_now().await;
		assert_eq!(count.0, 2);
		count.0 += 1;
	}
	app.world_mut().add_observer(my_observer).trigger(MyEvent);
	app.world_mut()
		.query_once::<&AsyncStreamTask>()
		.iter()
		.count()
		.xpect()
		.to_be(1);
	app.update();
	app.update();
	app.update();
	app.update();
	app.world_mut()
		.query_once::<&AsyncStreamTask>()
		.iter()
		.count()
		.xpect()
		.to_be(0);
	app.world_mut().resource::<Count>().0.xpect().to_be(3);
}
#[test]
fn streams() {
	let mut app = App::new();
	app.insert_resource(Count(0))
		.add_plugins((MinimalPlugins, AsyncPlugin));

	#[async_system]
	async fn my_system(mut count: ResMut<Count>) {
		let mut stream = StreamCounter::new(3);
		while let index = stream.next().await {
			{
				let _ = future::yield_now().await;
			}
			assert_eq!(index, count.0);
			count.0 += 1;
		}
	}

	app.world_mut().run_system_cached(my_system).ok();
	app.world_mut()
		.query_once::<&AsyncStreamTask>()
		.iter()
		.count()
		.xpect()
		.to_be(1);
	app.update();
	app.update();
	app.world_mut()
		.query_once::<&AsyncStreamTask>()
		.iter()
		.count()
		.xpect()
		.to_be(0);
	app.world_mut().resource::<Count>().0.xpect().to_be(3);
}

#[sweet::test]
async fn returns_value_future() {
	#[async_system]
	async fn my_system(mut count: ResMut<Count>) -> usize {
		let _ = future::yield_now().await;
		let before = count.0;
		count.0 += 5;
		if count.0 == 1 {
			let _ = future::yield_now().await;
			return count.0;
		}
		let _ = future::yield_now().await;
		// return before + count.0;
		before + count.0
	}

	let mut app = App::new();
	app.insert_resource(Count(10))
		.add_plugins((MinimalPlugins, AsyncPlugin));

	let fut = app.world_mut().run_system_cached(my_system).unwrap();
	app.world_mut()
		.query_once::<&AsyncStreamTask>()
		.iter()
		.count()
		.xpect()
		.to_be(1);

	// Progress async work to completion
	app.update();
	app.update();

	fut.await.xpect().to_be(25);
	// After completion, the stream task should be removed
	app.world_mut()
		.query_once::<&AsyncStreamTask>()
		.iter()
		.count()
		.xpect()
		.to_be(0);
}

#[sweet::test]
async fn complex() {
	/// an async system using futures and streams to count to five
	#[async_system]
	async fn my_system(mut count: ResMut<Count>) -> usize {
		future::yield_now().await;
		count.0 += 1;
		assert_eq!(count.0, 1);
		while let index = StreamCounter::new(4).await {
			assert_eq!(count.0, index + 1);
			count.0 += 1;
		}
		assert_eq!(count.0, 5);
		count.0
	}
	let mut app = App::new();
	app.insert_resource(Count(0))
		.add_plugins((MinimalPlugins, AsyncPlugin));

	let fut = app.world_mut().run_system_cached(my_system).unwrap();
	app.world_mut()
		.query_once::<&AsyncStreamTask>()
		.iter()
		.count()
		.xpect()
		.to_be(1);

	// Progress async work to completion
	app.update();
	app.update();

	fut.await.xpect().to_be(5);
	// After completion, the stream task should be removed
	app.world_mut()
		.query_once::<&AsyncStreamTask>()
		.iter()
		.count()
		.xpect()
		.to_be(0);
}
#[sweet::test]
async fn results() {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AsyncPlugin));

	#[async_system]
	async fn my_system() -> Result {
		let _ = Ok(())?;
		let _ = future::yield_now().await;
		// let _ = async move { Ok(()) }.await?;
		let _ = future::yield_now().await;
		{
			let _ = Err("foobar".into())?;
			let _ = future::yield_now().await;
		}
		let _ = future::yield_now().await;
		let _ = Ok(())?;
		Ok(())
	}

	let fut = app.world_mut().run_system_cached(my_system).unwrap();
	app.update();
	app.update();
	app.world_mut()
		.query_once::<&AsyncStreamTask>()
		.iter()
		.count()
		.xpect()
		.to_be(0);
	fut.await.unwrap_err().to_string().xpect().to_be("foobar\n");
}
