//! # Templating
//!
//! This example builds on the server example, demonstrates basic templating.
//! It loads assets and uses string replacement to create dynamic html pages.
//! Assets are loaded on each request so can be modified without restarting the server.
//!
//! For a full routing example see the `http_router` example which uses `beet_router`.
//!
//! ## Running the Example
//!
//! ```sh
//! cargo run --example templating --features http_server
//! ```
//!
//! Then test it with:
//! ```sh
//! curl http://localhost:8337
//! curl http://localhost:8337/planting-trees
//! curl http://localhost:8337/foobar
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
				Count::default(),
				handler_exchange(router),
			));
		})
		.run();
}

#[derive(Default, Component)]
struct Count(u32);

/// A simple router implementation that matches the request path
fn router(entity: EntityWorldMut, request: Request) -> Response {
	println!("{}: {}", request.method(), request.path_string());

	let route = match request.path_string().as_str() {
		"/" => home,
		"/planting-trees" => planting_trees,
		_ => not_found,
	};
	route(entity, request)
}



/// Home page.
fn home(mut entity: EntityWorldMut, _: Request) -> Response {
	let mut count = entity.get_mut::<Count>().unwrap();
	count.0 += 1;
	Response::ok_body(
		render(&format!(
			r#"
<h1>🌱 The Garden Bed 🌱</h1>
<p>The <i>number one</i> place for great gardening information.</p>
<br/>
<p>Greetings visitor {}</p>
<p>Visit a link or check out a <a href="/foobar">broken link</a>
"#,
			count.0
		)),
		"text/html",
	)
}

/// Planting trees page.
fn planting_trees(_: EntityWorldMut, _: Request) -> Response {
	Response::ok_body(
		render(
			r#"
<h1>Planting Trees</h1>
<p>Do it, just do it. Dont ask questions. Go and buy a native tree and plant it somewhere.</p>
"#,
		),
		"text/html",
	)
}

/// 404 handler
fn not_found(_: EntityWorldMut, request: Request) -> Response {
	let path = request.path_string();
	Response::from_status_body(
		StatusCode::NotFound,
		render(&format!(
			r#"
<h1>Not Found</h1>
<p>The path at <a href="{path}">{path}</a> could not be found.</p>
"#,
		)),
		"text/html",
	)
}

fn nav_items() -> &'static [(&'static str, &'static str)] {
	&[("Home", "/"), ("Planting Trees", "/planting-trees")]
}

/// ╔═══════════════════════════════════════════╗
/// ║   Basic Templating Engine                 ║
/// ╚═══════════════════════════════════════════╝

/// Template rendering function that wraps content in a layout.
///
/// Loads the layout from disk and injects head, navigation, and main content.
/// Since this loads from disk on every request, you can edit the HTML files
/// without restarting the server!
fn render(main_content: &str) -> String {
	use std::fs::read_to_string;

	read_to_string("examples/assets/layouts/default-layout.html")
		.unwrap()
		.replace("{{ head }}", &head())
		.replace("{{ nav }}", &nav())
		.replace("{{ main }}", &main_content)
}

/// Generates the `<head>` content for pages.
///
/// Includes JavaScript for theme switching functionality.
fn head() -> String {
	use std::fs::read_to_string;

	let theme_switcher =
		read_to_string("examples/assets/js/minimal-theme-switcher.js").unwrap();

	format!(r#"<script>{theme_switcher}</script>"#)
}

/// Generates navigation HTML from a list of nav items.
fn nav() -> String {
	nav_items()
		.iter()
		.map(|(label, path)| {
			format!(r#"<li><a href="{}">{}</a></li>"#, path, label)
		})
		.collect::<String>()
}
