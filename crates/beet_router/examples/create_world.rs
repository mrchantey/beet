//! A bench to check that creating a world is acceptable to do for every request.
//! 
//! I get about 0.1ms average world creation time, seeing as several milliseconds is
//! acceptable for a request, this seems an acceptable baseline.
use bevy::app::PluginsState;
use bevy::prelude::*;



fn main() {
	let mut total_duration = std::time::Duration::ZERO;
	let iterations = 10_000;
	
	for iteration in 0..iterations {
		let now = std::time::Instant::now();
		create_app();
		total_duration += now.elapsed();
		
		if iteration % 1000 == 0 {
			println!("Completed {} iterations", iteration);
		}
	}
	
	let average_duration = total_duration / iterations;
	println!("Average app creation time over {} iterations: {:?}", iterations, average_duration);
	println!(
		"Total entity count: {}",
		COUNTER.load(std::sync::atomic::Ordering::Relaxed)
	);

}

static COUNTER: std::sync::atomic::AtomicUsize =
	std::sync::atomic::AtomicUsize::new(0);



fn create_app() {
	let mut app = App::new();
	app.add_plugins(|app: &mut App| {
		app.world_mut();
		app.add_systems(Startup, |mut commands: Commands| {
			commands.spawn(Name::new("Main World"));
			// println!("boom");
		});
		app.add_systems(Update, |names: Query<&Name>| {
			// println!("bang");
			for _name in names.iter() {
				COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
			}
		});
	})
	.set_runner(run_once)
	.run();
}

// from bevy_app https://github.com/mrchantey/bevy/blob/a1f4e56610c090b44f8b4a8f3eb56aeda5eb9669/crates/bevy_app/src/app.rs#L1392
fn run_once(mut app: App) -> AppExit {
	while app.plugins_state() == PluginsState::Adding {
		#[cfg(not(target_arch = "wasm32"))]
		bevy::tasks::tick_global_task_pools_on_main_thread();
	}
	app.finish();
	app.cleanup();

	app.update();

	app.should_exit().unwrap_or(AppExit::Success)
}
