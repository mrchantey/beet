//! Server benchmark.
//!
//! Measures roundtrip latency for HTTP requests to a minimal server.
//!
//! Run with:
//! ```sh
//! cargo bench --bench server_bench --features http_server,ureq
//! ```
use beet::prelude::*;

fn main() { async_ext::block_on(main_async()); }

async fn main_async() {
	let server = HttpServer::new(HttpServer::free_port().unwrap());
	let url = server.local_url();
	std::thread::spawn(|| {
		App::new()
			.add_plugins((MinimalPlugins, ServerPlugin::default()))
			.spawn((
				server,
				CallOnReady::on_spawn(),
				children![exchange_ext::handler(|_| {
					Response::ok_body("hello world", MediaType::Text)
				})],
			))
			.run();
	});

	time_ext::sleep_millis(10).await;
	let start = Instant::now();

	// let num_requests = 1; //     700 us
	// let num_requests = 10; //    850 us
	// let num_requests = 100; //   5 ms
	let num_requests = 1000; //     65 ms

	let futures = (0..num_requests).map(|_| async {
		let start = Instant::now();
		Request::get(&url).send().await.unwrap();
		start.elapsed()
	});
	let durations = futures::future::join_all(futures).await;
	let avg = durations.iter().sum::<Duration>() / (durations.len() as u32);

	cross_log!(
		"Complete:\n  requests: {}\n  avg: {}\n  total: {}",
		num_requests,
		time_ext::pretty_print_duration(avg),
		time_ext::pretty_print_duration(start.elapsed())
	);
}
