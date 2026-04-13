//! # Templating
//!
//! This example builds on the server example, demonstrates basic templating.
//! It loads assets and uses string replacement to create dynamic html pages.
//! Assets are loaded on each request so can be modified without restarting the server.
//!
//! For a full routing example see the `router` example which uses `beet_router`.
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
				Action::<Request, Response>::new_system(router),
			));
		})
		.run();
}

#[derive(Default, Component)]
struct Count(u32);

/// A simple router implementation that matches the request path
fn router(
	In(cx): In<ActionContext<Request>>,
	mut counts: Query<&mut Count>,
) -> Result<Response> {
	let entity = cx.caller.id();
	let request = cx.input;
	println!("{}: {}", request.method(), request.path_string());

	match request.path_string().as_str() {
		"/" => {
			let mut count = counts.get_mut(entity).unwrap();
			count.0 += 1;
			home(count.0)
		}
		"/planting-trees" => planting_trees(),
		_ => not_found(&request),
	}
	.xok()
}



/// Home page.
fn home(visitor_number: u32) -> Response {
	Response::ok_body(
		render(&format!(
			r#"
<h1>🌱 The Garden Bed 🌱</h1>
<p>The <i>number one</i> place for great gardening information.</p>
<br/>
<p>Greetings visitor {}</p>
<p>Visit a link or check out a <a href="/foobar">broken link</a>
"#,
			visitor_number
		)),
		MimeType::Html,
	)
}

/// Planting trees page.
fn planting_trees() -> Response {
	Response::ok_body(
		render(
			r#"
<h1>Planting Trees</h1>
<p>Do it, just do it. Dont ask questions. Go and buy a native tree and plant it somewhere.</p>
"#,
		),
		MimeType::Html,
	)
}

/// 404 handler
fn not_found(request: &Request) -> Response {
	let path = request.path_string();
	Response::from_status_body(
		StatusCode::NOT_FOUND,
		render(&format!(
			r#"
<h1>Not Found</h1>
<p>The path at <a href="{path}">{path}</a> could not be found.</p>
"#,
		)),
		MimeType::Html,
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
