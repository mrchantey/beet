use beet_core::prelude::*;
use beet_utils::time_ext;
use bevy::prelude::*;
use std::time::Duration;

#[tokio::main]
#[rustfmt::skip]
async fn main() {
	App::new()
		.add_plugins((MinimalPlugins, AsyncPlugin))
		.add_systems(Startup, my_func).run();
}

fn my_func(mut spawn_async: SpawnAsync, mut commands: Commands) {
	commands.spawn(Name::new("hello world"));
	spawn_async.spawn_and_run_async(async move {
		let simulated_value = time_ext::sleep(Duration::from_secs(1)).await;
		move |mut spawn_async: SpawnAsync, mut query: Query<&mut Name>| {
			let mut item = query.single_mut().unwrap();
			println!("name is now {:?}", item);
			println!("simulated value: {:?}", simulated_value);
			*item = "foobar".into();
			spawn_async.spawn_and_run_async(async move {
				time_ext::sleep(Duration::from_secs(1)).await;
				move |query: Query<&Name>| {
					println!("name is finally {:?}", query.single().unwrap());
				}
			})
		}
	});
}

// async fn my_ideal_func(mut commands: Commands, mut query: Query<&mut Name>) {
// 	commands.spawn(Name::new("hello world"));
// 	let simulated_value = time_ext::sleep(Duration::from_secs(1)).await;
// 	let mut item = query.single_mut().unwrap();
// 	println!("name is now {:?}", item);
// 	println!("simulated value: {:?}", simulated_value);
// 	*item = "foobar".into();
// 	time_ext::sleep(Duration::from_secs(1)).await;
// 	println!("name is finally {:?}", query.single().unwrap());
// }
