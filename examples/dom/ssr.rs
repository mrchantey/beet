//! An example of basic server-side rendering (SSR).
use axum::Router;
use axum::extract::Query as QueryParams;
use axum::extract::State;
use beet::prelude::*;
use bevy::prelude::*;
#[tokio::main]
async fn main() {
	let app = Router::<AppState>::new()
		.bundle_route("/", my_route)
		.with_state(AppState {
			started: std::time::Instant::now(),
		});

	let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
		.await
		.unwrap();
	println!("listening on {}", listener.local_addr().unwrap());
	axum::serve(listener, app).await.unwrap();
}

#[derive(serde::Deserialize)]
struct RequestPayload {
	name: Option<String>,
}

#[derive(Clone)]
struct AppState {
	started: std::time::Instant,
}

fn my_route(
	state: State<AppState>,
	payload: QueryParams<RequestPayload>,
) -> impl Bundle {
	let name = payload.name.clone().unwrap_or("world".to_string());
	let now = state.started.elapsed();
	let uptime = format!("{}.{:03}", now.as_secs(), now.subsec_millis());
	rsx! {
		<body>
			<h1>Hello {name}!</h1>
			<p>uptime: {uptime}</p>
			<p>"use the 'name' query params to receive a warm greeting!"</p>
		</body>
	}
}
