//! Server benchmark example.
//!
//! Measures roundtrip latency for HTTP requests to a minimal server.
//!
//! Run with:
//! ```sh
//! cargo run --example server_bench --features server_app
//! ```
use beet::prelude::*;


fn main() { async_ext::block_on(main_async()); }
async fn main_async() {
	let _handle = std::thread::spawn(|| {
		App::new()
			.add_plugins((MinimalPlugins, ServerPlugin::default()))
			.add_observer(|ev: On<Insert, Request>, mut commands: Commands| {
				commands
					.entity(ev.event_target())
					.insert(Response::ok_body("hello world", "text/plain"));
			})
			.run();
	});

	time_ext::sleep_millis(10).await;
	let start = std::time::Instant::now();

	// let num_requests = 1; //     700 us
	// let num_requests = 10; //    850 us
	// let num_requests = 100; //   5 ms
	let num_requests = 1000; //     65 ms


	let futures = (0..num_requests).map(|_| async {
		let start = Instant::now();
		Request::get(DEFAULT_SERVER_LOCAL_URL).send().await.unwrap();
		start.elapsed()
	});
	let durations = futures::future::join_all(futures).await;
	let avg = durations.iter().sum::<std::time::Duration>()
		/ (durations.len() as u32);

	let total = start.elapsed();
	println!(
		"Complete:\n  requests: {}\n  avg: {}\n  total: {}",
		num_requests,
		time_ext::pretty_print_duration(avg),
		time_ext::pretty_print_duration(total)
	);

	// let mut
}
