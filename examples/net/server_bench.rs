//! A minimal server example
use beet::prelude::*;
use bevy::log::LogPlugin;

fn main() {
	let _handle = std::thread::spawn(|| {
		App::new()
			.add_plugins((
				MinimalPlugins,
				LogPlugin::default(),
				ServerPlugin::default(),
			))
			.add_observer(|ev: On<Insert, Request>, mut commands: Commands| {
				commands
					.entity(ev.event_target())
					.insert(Response::ok_body("hello world", "text/plain"));
			})
			.run();
	});

	let start = std::time::Instant::now();

	let num_requests = 1000;
	let store = Store::new_vec();

	async_executor::LocalExecutor::new().run(async {
		let mut handles = Vec::new();
		let futures = (0..num_requests).map(|_| async {
			let start = Instant::now();
			Request::get("http://127.0.0.1:8337").send().await.unwrap();
			start.elapsed()
		});
		join_all(futures).await
	});

	let duration = start.elapsed();
	let avg = store.get().iter().sum::<std::time::Duration>()
		/ (store.get().len() as u32);
	println!(
		"Complete:\n  requests: {}\n  avg: {}\n  total: {}",
		num_requests,
		time_ext::pretty_print_duration(avg),
		time_ext::pretty_print_duration(duration)
	);

	// let mut
}
