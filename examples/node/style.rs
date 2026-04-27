//! # Basic Server
//!
//! This example demonstrates creating a basic server in Beet
//! using [`Action::new_pure`], the simplest synchronous exchange pattern.
//! For concurrent request handling see [`spawn_exchange`].
//!
//!
//! ## Running the Example
//!
//! ```sh
//! cargo run --example server --features http_server
//! # or with CliServer
//! cargo run --example server --features http_server -- --name=billy
//! ```
//!
//! Then test it with:
//! ```sh
//! curl http://localhost:8337
//! curl http://localhost:8337?name=billy
//! ```
//!
//! Try refreshing multiple times to see the visitor count increment!
//!
use beet::prelude::style::*;
use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			ServerPlugin::default(),
			material::MaterialStylePlugin::new(palettes::basic::RED),
		))
		.add_systems(Startup, |mut commands: Commands| {
			commands.spawn((
				HttpServer::default(),
				Action::<Request, Response>::new_system(handler),
			));
		})
		.run();
}

/// Handler function that processes all incoming requests.
fn handler(
	cx: In<ActionContext<Request>>,
	query: StyleQuery,
) -> Result<Response> {
	let css = query.build_css(
		&CssBuilder::default().with_format_variables(FormatVariables::Full),
		cx.id(),
	)?;

	let html = format!(
		r#"
<!DOCTYPE html>
<html>
<head>
	<title>Beet Style Example</title>
	<link
		rel="stylesheet"
		href="https://unpkg.com/tailwindcss@4/preflight.css"
	/>
	<style>{css}</style>
</head>
<body>
<header>Hello World</header>
</body>
"#
	);

	Response::ok_body(html, MimeType::Html).xok()
}
