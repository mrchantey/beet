//! Minimal Lambda test binary used by integration tests.
//! Returns a version marker that tests can verify after deployment.
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Test version marker. Integration tests modify this value
/// to verify binary updates across deploys.
const TEST_VERSION: &str = "test-v1";

fn main() -> AppExit {
	App::new()
		.add_plugins((MinimalPlugins, ServerPlugin))
		.spawn_then((
			HttpServer::default(),
			exchange_handler(handle_request),
		))
		.run()
}

fn handle_request(req: ActionContext<Request>) -> Response {
	let req = req.take();
	match req.path_string().as_str() {
		"/version" => Response::ok().with_body(TEST_VERSION),
		"/health" => Response::ok().with_body("ok"),
		_ => Response::ok().with_body(format!("lambda-test:{TEST_VERSION}")),
	}
}
