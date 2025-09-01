use beet_core::prelude::*;
use beet_utils::time_ext;
use bevy::prelude::*;
use std::time::Duration;

#[tokio::main]
#[rustfmt::skip]
async fn main() {
	App::new()
		.add_plugins((MinimalPlugins, AsyncPlugin))
		// .add_systems(Startup, my_func)
		.add_systems(Startup, my_system)
		.run();
}

#[async_system]
async fn my_system(mut commands: Commands, mut query: Query<&mut Name>) {
	commands.spawn(Name::new("hello world"));
	let simulated_value = time_ext::sleep(Duration::from_secs(1)).await;
	let mut item = query.single_mut().unwrap();
	println!("name is now {:?}", item);
	println!("simulated value: {:?}", simulated_value);
	*item = "foobar".into();
	time_ext::sleep(Duration::from_secs(1)).await;
	println!("name is finally {:?}", query.single().unwrap());
}

#[allow(unused_mut, unused_variables)]
fn _my_system_expanded(
	mut __async_commands: AsyncCommands,
	mut commands: Commands,
) {
	commands.spawn(Name::new("hello world"));
	__async_commands.spawn_and_run(async move {
		let simulated_value = time_ext::sleep(Duration::from_secs(1)).await;
		move |mut __async_commands: AsyncCommands,
		      mut query: Query<&mut Name>| {
			let mut item = query.single_mut().unwrap();
			println!("name is now {:?}", item);
			println!("simulated value: {:?}", simulated_value);
			*item = "foobar".into();
			__async_commands.spawn_and_run(async move {
				time_ext::sleep(Duration::from_secs(1)).await;
				move |query: Query<&Name>| {
					println!("name is finally {:?}", query.single().unwrap());
				}
			})
		}
	});
}
