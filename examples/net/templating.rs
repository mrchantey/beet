//! An intermediate HTTP server using `flow_exchange`
//! to respond to all requests. A basic understanding of `beet_flow` is recommended.
//!
//! Demonstrates hand-rolled routing and templating, note that this is
//! usually handled by `beet_router`.
//!
//! Run with:
//! ```sh
//! cargo run --example server_templating --features http_server
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
			commands.spawn((HttpServer::default(), handler_exchange(handler)));
		})
		.run();
}

fn handler(_: EntityWorldMut, request: Request) -> Response {
	let route = match request.path_string().as_str() {
		"/" => home,
		"/planting-trees" => planting_trees,
		_ => not_found,
	};
	route(request)
}



fn home(_: Request) -> Response {
	Response::ok_body(
		render(
			r#"
<h1>Unbeetable Gardening Tips</h1>
<p>The number one place for great gardening information.</p>
<p>Visit a link or check out a <a href="/foobar">broken link</a>
"#,
		),
		"text/html",
	)
}
fn planting_trees(_: Request) -> Response {
	Response::ok_body(
		render(
			r#"
<h1>Planting Trees</h1>
<p>Do it, just do it. Dont ask questions. Go and buy a tree and plant it somewhere.</p>
"#,
		),
		"text/html",
	)
}

fn not_found(_: Request) -> Response {
	Response::from_status_body(
		StatusCode::NotFound,
		render(
			r#"
<h1>Not Found</h1>
<p>The path could not be found.</p>
"#,
		),
		"text/html",
	)
}



// renders a page with navigation and content.
// This is loaded on every request, so you can change the html without reload!
fn render(main_content: &str) -> String {
	use std::fs::read_to_string;

	read_to_string("examples/assets/layouts/default-layout.html")
		.unwrap()
		.replace("{{ head }}", &head())
		.replace("{{ nav }}", &nav())
		.replace("{{ main }}", &main_content)
}

fn head() -> String {
	use std::fs::read_to_string;

	let theme_switcher =
		read_to_string("examples/assets/js/minimal-theme-switcher.js").unwrap();

	format!(r#"<script>{theme_switcher}</script>"#)
}

fn nav() -> String {
	let nav_items = [("Home", "/"), ("Planting Trees", "/planting-trees")];
	nav_items
		.iter()
		.map(|(label, path)| {
			format!(r#"<li><a href="{}">{}</a></li>"#, path, label)
		})
		.collect::<String>()
}
