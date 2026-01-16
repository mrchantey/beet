//! Example of a basic server with hand-rolled routing and templating
//! using ExchangeSpawner for request handling.
use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			ServerPlugin::default(),
		))
		.add_systems(Startup, |mut commands: Commands| {
			commands.spawn((
				HttpServer::default(),
				ExchangeSpawner::new_flow(|| {
					// Use InfallibleSequence to run all endpoints
					(InfallibleSequence, children![
						EndpointBuilder::get().with_handler(home),
						EndpointBuilder::get()
							.with_path("foo")
							.with_handler(foo),
						// Catch-all for not found
						EndpointBuilder::get()
							.with_predicate(common_predicates::no_response())
							.with_trailing_path()
							.with_handler(not_found),
					])
				}),
			));
		})
		.run();
}
fn layout(body: &str) -> String {
	format!(
		r#"
<!DOCTYPE html>
<html>
		<head>
				<title>Beet Server</title>
				<style>
						body {{
								font-family: system-ui, sans-serif;
								background-color: black;
								color: white;
						}}
				</style>
		</head>
		<body>
				{body}
		</body>
</html>
"#,
	)
}

fn home() -> Response {
	Response::ok_body(
		layout(
			"<h1>Hello from Beet!</h1><p>This is a simple server example.</p><p>Try visiting <a href=\"/foo\">/foo</a> for another page.</p>",
		),
		"text/html",
	)
}

fn foo() -> Response {
	Response::ok_body(
		layout("<h1>Hello Foo!</h1><a href='/'>Back home</a>"),
		"text/html",
	)
}

fn not_found() -> Response {
	Response::from_status_body(
		StatusCode::NotFound,
		layout("<h1>Whoops! page not found</h1>"),
		"text/html",
	)
}
