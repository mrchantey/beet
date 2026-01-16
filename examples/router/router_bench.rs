//! Benches a large amount of nested branches
//! In practice this is quite a large tree,
//! a well formed router should break much earlier.
//! That said for a 200ms request this is unnoticable
//!
//! short path: 			30us
//! 100 nested paths: 130us
//!
#![recursion_limit = "1024"]
use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			RouterPlugin::default(),
		))
		.add_systems(Startup, |mut commands: Commands| {
			commands.spawn((
				HttpServer::default(),
				ExchangeSpawner::new_flow(|| {
					(InfallibleSequence, children![
						EndpointBuilder::get().with_handler(|| {
							Response::ok_body(
								r#"
								<h1>home</h1>
								<a href="/nested"> visit bench </a>
								<a href="/status"> visit status </a>
								"#,
								"text/html",
							)
						}),
						// This will be the fastest, as its a constant value we can do
						// a lot less async channel stuff
						EndpointBuilder::get().with_path("/status"),
						// Benches a large amount of nested branches,
						// adding ~100us of latency.
						// In practice this is quite a large tree,
						// a well formed router should break much earlier.
						// That said for a 200ms request this is unnoticable
						EndpointBuilder::get()
							.with_path("nested")
							.with_handler_bundle(nested_sequence(
								(|| Response::ok_body(
									r#"
									<h1>bench</h1>
									<a href="/"> visit home </a>"#,
									"text/html",
								))
								.into_endpoint_handler()
							),)
					])
				}),
			));
		})
		.run();
}

/// 100 nested sequences
#[rustfmt::skip]
fn nested_sequence(inner: impl Bundle) -> impl Bundle {
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![
(Sequence, children![inner])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
])
}
