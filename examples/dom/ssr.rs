//! An example of basic server-side rendering (SSR) with beet.
//!
//! ```sh
//! cargo run --example ssr --features=server
//! ```
use axum::Router;
use axum::extract::Query as QueryParams;
use axum::extract::State;
use beet::prelude::*;

fn main() {
	AppRouter::<AppState>::new(AppState {
		started: std::time::Instant::now(),
	})
	.bundle_route("/", my_route)
	.serve();
}

#[derive(Clone)]
struct AppState {
	started: std::time::Instant,
}

#[derive(serde::Deserialize)]
struct RequestPayload {
	name: Option<String>,
}

/// A [`BundleRoute`] is a regular axum route that returns a [`Bundle`].
fn my_route(
	state: State<AppState>,
	payload: QueryParams<RequestPayload>,
) -> impl Bundle {
	let name = payload.name.clone().unwrap_or("world".to_string());
	let now = state.started.elapsed();
	let uptime = format!("{:.2}", now.as_secs_f32());
	rsx! {
		<WarmGreeting name=name/>
		<p>uptime: {uptime} seconds</p>
	}
}


#[template]
fn WarmGreeting(name: String) -> impl Bundle {
	rsx! {
		<div>
			<h1>Hello {name}!</h1>
		</div>
	}
}
