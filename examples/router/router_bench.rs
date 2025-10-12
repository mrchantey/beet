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
			FlowRouterPlugin::default(),
		))
		.add_systems(Startup, |mut commands: Commands| {
			commands.spawn((RouteServer, InfallibleSequence, children![
				endpoint(
					HttpMethod::Get,
					handler(|| Response::ok_body(
						r#"
						<h1>home</h1>
						<a href="/nested"> visit bench </a>
						"#,
						"text/html"
					)),
				),
				// Benches a large amount of nested branches
				// In practice this is quite a large tree,
				// a well formed router should break much earlier.
				// That said for a 200ms request this is unnoticable
				//
				// short path: 			30us
				// 100 nested paths: 130us
				parse_path_filter(
					PathFilter::new("nested"),
					endpoint(
						HttpMethod::Get,
						nested_sequence(handler(|| Response::ok_body(
							r#"
							<h1>bench</h1>
							<a href="/"> visit home </a>"#,
							"text/html"
						))),
					),
				)
			]));
		})
		.run();
}


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
