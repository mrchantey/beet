//! # HTTP Router
//!
//! 🚧 Mind Your Step! 🚧
//! `beet_router` is under construction, sharp edges ahoy.
//!
//! This example builds on the templating example,
//! demonstrating proper routing and control flow using `beet_router`.
//!
//! `beet_router` uses the control flow primitives from `beet_flow` for routing,
//! see the flow examples for more detailed usage.
//!
//! ## Running the Example
//!
//! ```sh
//! cargo run --example http_router --features server_app
//! ```
//!
//! Then test it with:
//! ```sh
//! curl http://localhost:5000
//! curl http://localhost:5000/mashed-beets
//! curl http://localhost:5000/foobar
//! ```
use beet::prelude::*;
fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			RouterPlugin::default(), // provides routing infrastructure
		))
		.add_systems(Startup, |mut commands: Commands| {
			commands.spawn((
				HttpServer::default(),
				// Count::default(),
				// flow_exchange runs control flow on each request
				flow_exchange(|| {
					(InfallibleSequence, children![
						log_stats(),    // runs first, logs request then passes
						home(),         // matches GET /
						mashed_beets(), // matches GET /treasure
						not_found()     // only runs if no response yet
					])
				}),
			));
		})
		.run();
}

/// Logging middleware using an observer.
///
/// This runs before each action in the flow, logging the request details
/// then passing control to the next action with [`Outcome::Pass`].
///
/// This pattern is useful for cross-cutting concerns like logging, metrics, or auth.
fn log_stats() -> impl Bundle {
	OnSpawn::observe(
		|ev: On<GetOutcome>,
		 mut commands: Commands,
		 query: RouteQuery|
		 -> Result {
			let action = ev.target();
			let request = query.request_meta(action)?;
			println!("{}: {}", request.method(), request.path_string());
			// pass to next action in the sequence
			commands.entity(action).trigger_target(Outcome::Pass);
			Ok(())
		},
	)
}

/// Home endpoint handling GET requests to `/`.
fn home() -> impl Bundle {
	EndpointBuilder::get().with_action(|| {
		Response::ok_body(
			render(
				r#"
<h1>🛖 The Food Hut 🛖</h1>
<p>The <i>number one</i> place for delicious food recipes.</p>
<br/>
<p>Visit a link or check out a <a href="/foobar">broken link</a>
"#,
			),
			"text/html",
		)
	})
}
/// Mashed Beets page.
fn mashed_beets() -> impl Bundle {
	EndpointBuilder::get()
		.with_path("mashed-beets")
		.with_action(|| {
			Response::ok_body(
				render(
					r#"
<h1>Mashed Beets</h1>
<p>Take a bunch of beets and beat them until they're mashed.</p>
"#,
				),
				"text/html",
			)
		})
}

/// 404 handler that only runs when no previous endpoint matched.
///
/// This demonstrates advanced control flow by combining:
/// 1. A predicate that checks if a response exists yet
/// 2. A non-canonical endpoint that accepts any remaining path
///
/// The sequence short-circuits if `no_response()` fails, meaning this
/// only runs when no previous endpoint has generated a response.
///
fn not_found() -> impl Bundle {
	(Name::new("Not Found"), Sequence, children![
		// predicate: only passes if no response exists
		common_predicates::no_response(),
		EndpointBuilder::new()
			// dont include in endpoint trees etc
			.non_canonical()
			// match any path
			.with_trailing_path()
			.with_action(|req: In<Request>| {
				let path = req.path_string();
				Response::from_status_body(
					StatusCode::NotFound,
					render(&format!(
						r#"
<h1>Not Found</h1>
<p>The page at <a href="{path}">{path}</a> could not be found.</p>"#
					)),
					"text/html",
				)
			})
	])
}

fn nav_items() -> &'static [(&'static str, &'static str)] {
	&[("Home", "/"), ("Mashed Beets", "/mashed-beets")]
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
