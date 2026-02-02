//! An intermediate HTTP server using `flow_exchange`
//! to respond to all requests. A basic understanding of `beet_flow` is recommended.
//!
//! Demonstrates hand-rolled routing and templating, note that this is
//! usually handled by `beet_router`.
//!
//! Run with:
//! ```sh
//! cargo run --example server_templating --features beet_net_server
//! ```
//!
//! Test with:
//! ```sh
//! curl http://localhost:8337
//! curl http://localhost:8337/foo
//! curl http://localhost:8337/does-not-exist
//! ```
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
				flow_exchange(|| {
					// Fallback will end on the first matching child.
					(Fallback, children![home(), foo(), not_found()])
				}),
			));
		})
		.run();
}


fn home() -> impl Bundle {
	(
		Name::new("Home"),
		endpoint(|req| {
			if req.path().is_empty() {
				Some(Response::ok_body(
					layout(
						"<h1>Hello from Beet!</h1><p>This is a simple server example.</p><p>Try visiting <a href=\"/foo\">/foo</a> for another page.</p>",
					),
					"text/html",
				))
			} else {
				None
			}
		}),
	)
}

fn foo() -> impl Bundle {
	(
		Name::new("Foo"),
		endpoint(|req| {
			if req.path_string() == "/foo" {
				Some(Response::ok_body(
					layout("<h1>Hello Foo!</h1><a href='/'>Back home</a>"),
					"text/html",
				))
			} else {
				None
			}
		}),
	)
}

fn not_found() -> impl Bundle {
	(
		Name::new("NotFound"),
		endpoint(|req| {
			let path = req.path_string();
			Some(Response::from_status_body(
				StatusCode::NotFound,
				layout(&format!("<h1>Whoops! '{path}' not found</h1>")),
				"text/html",
			))
		}),
	)
}


// basic endpoint control flow, this is usally done by beet_router.
fn endpoint<F>(handler: F) -> impl Bundle
where
	F: 'static + Send + Sync + Clone + Fn(&mut Request) -> Option<Response>,
{
	OnSpawn::observe(
		move |ev: On<GetOutcome>,
		      mut commands: Commands,
		      mut agent_query: AgentQuery<&mut Request>| {
			let action = ev.target();
			let agent = agent_query.entity(action);
			let mut request = agent_query.get_mut(action).unwrap();
			if let Some(response) = handler(&mut request) {
				commands.entity(agent).insert(response);
				commands.entity(action).trigger_target(Outcome::Pass);
			} else {
				commands.entity(action).trigger_target(Outcome::Fail);
			}
		},
	)
}

// basic html templating
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
