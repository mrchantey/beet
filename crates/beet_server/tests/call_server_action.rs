#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use axum::Json;
use axum::Router;
use axum::routing::get;
use axum::routing::post;
use beet_router::prelude::*;
use beet_server::prelude::*;
use sweet::prelude::*;
use tokio::net::TcpListener;
use tokio::spawn;

async fn add_handler_get(
	JsonQuery(params): JsonQuery<(i32, i32)>,
) -> Json<i32> {
	Json(params.0 + params.1)
}

async fn add_handler_post(Json(params): Json<(i32, i32)>) -> Json<i32> {
	Json(params.0 + params.1)
}

#[sweet::test]
async fn test_server_action_calls() {
	// Setup the server
	let app = Router::new()
		.route("/add", get(add_handler_get))
		.route("/add", post(add_handler_post));

	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();

	// Start the server in a separate task
	let _server = spawn(async move {
		axum::serve(listener, app).await.unwrap();
	});
	// Set the server URL to point to our test server
	CallServerAction::set_server_url(RoutePath::new(format!(
		"http://{}",
		addr
	)));

	// Test GET request
	expect(
		CallServerAction::request::<_, i32>("get", "/add", (5, 3))
			.await
			.unwrap(),
	)
	.to_be(8);

	// Test POST request
	expect(
		CallServerAction::request::<_, i32>("post", "/add", (10, 7))
			.await
			.unwrap(),
	)
	.to_be(17);

	// We don't need to explicitly shut down the server as it will be dropped
	// when the test completes
}
