//! HTTP router example with a pirate theme.
//!
//! Demonstrates a basic HTTP router using `beet_router` for request routing.
//!
//! Run with:
//! ```sh
//! cargo run --example http_router --features server_app
//! ```
//!
//! Test with:
//! ```sh
//! curl http://localhost:5000
//! curl http://localhost:5000/treasure
//! curl http://localhost:5000/plank
//! ```
use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			RouterPlugin::default(),
		))
		.add_systems(Startup, |mut commands: Commands| {
			commands.spawn((
				// CliServer,
				HttpServer::default(),
				flow_exchange(|| {
					(InfallibleSequence, children![
						log_stats(),
						home(),
						treasure(),
						not_found()
					])
				}),
			));
		})
		.run();
}

fn log_stats() -> impl Bundle {
	OnSpawn::observe(
		|ev: On<GetOutcome>,
		 mut commands: Commands,
		 query: RouteQuery|
		 -> Result {
			let action = ev.target();
			let request = query.request_meta(action)?;
			println!("{}: {}", request.method(), request.path_string());
			commands.entity(action).trigger_target(Outcome::Pass);
			Ok(())
		},
	)
}

fn home() -> impl Bundle {
	EndpointBuilder::get().with_action(|| {
		Response::ok_body(
			render(
				r#"
<h1>Beet cooking tips!</h1>
<p>Welcome to the pirate's cove, ye scallywag!</p>
<p>Visit a link or check out a <a href="/foobar">broken link</a>
"#,
			),
			"text/html",
		)
	})
}

fn treasure() -> impl Bundle {
	EndpointBuilder::get()
		.with_path("treasure")
		.with_action(|| {
			Response::ok_body(
				render(
					r#"
<h1>Treasure Map</h1>
<p>X marks the spot where the booty be buried, matey!</p>
"#,
				),
				"text/html",
			)
		})
}

/// # Not Found Handler
///
/// This handler is an example of custom routing logic,
/// using control flow primitives to only run this endpoint if no response already.
///
/// It will only run if, after all endpoints have ran there is still no response.
/// If there is a response the predicate will fail, short-circuting the sequence.
///
/// [`EndpointBuilder`] exists to create control flow structures like this.
fn not_found() -> impl Bundle {
	(Name::new("Not Found"), Sequence, children![
		// only passes if no response is present,
		// indicating no previous endpoint has ran
		common_predicates::no_response(),
		EndpointBuilder::new()
			.non_canonical()
			.with_trailing_path()
			.with_action(|req: In<Request>| {
				let path = req.path_string();
				Response::from_status_body(
					StatusCode::NotFound,
					render(&format!(
						r#"
<h1>Not Found</h1>
<p>Arrr! The page at <a href="{path}">{path}</a> has been sent to Davy Jones' locker.</p>"#
					)),
					"text/html",
				)
			})
	])
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
	let nav_items = [("Home", "/"), ("Treasure", "/treasure")];
	nav_items
		.iter()
		.map(|(label, path)| {
			format!(r#"<li><a href="{}">{}</a></li>"#, path, label)
		})
		.collect::<String>()
}
