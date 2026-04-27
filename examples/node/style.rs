//! # Material Design Style Example
//!
//! This example demonstrates the Material Design 3 styling system in Beet,
//! including buttons, cards, typography, and layout components.
//!
//! ## Running the Example
//!
//! ```sh
//! cargo run --example style --features http_server
//! ```
//!
//! Then open your browser to:
//! ```
//! http://localhost:8337
//! ```
//!
use beet::prelude::style::*;
use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			ServerPlugin::default(),
			material::MaterialStylePlugin::new(palettes::basic::YELLOW),
		))
		.add_systems(Startup, |mut commands: Commands| {
			commands.spawn((
				HttpServer::default(),
				Action::<Request, Response>::new_system(handler),
			));
		})
		.run();
}

/// Handler function that generates a Material Design 3 styled page.
fn handler(
	cx: In<ActionContext<Request>>,
	query: StyleQuery,
) -> Result<Response> {
	let css = query.build_css(
		&CssBuilder::default()
			// .with_format_variables(FormatVariables::Full),
			// .with_format_variables(FormatVariables::Hash { min_len: 1 }),
			.with_format_variables(FormatVariables::short()),
		cx.id(),
	)?;

	let html = format!(
		r#"<!DOCTYPE html>
<html>
<head>
	<meta charset="utf-8">
	<meta name="viewport" content="width=device-width, initial-scale=1">
	<title>Beet Style</title>
	<link
		rel="stylesheet"
		href="https://unpkg.com/tailwindcss@4/preflight.css"
	/>
	<style>{css}</style>
</head>
<body>
	<header>Hello World!</header>
</body>
</html>"#
	);

	fs_ext::write(
		AbsPathBuf::new_workspace_rel("target/examples/style/index.html")
			.unwrap(),
		&html,
	)?;

	Response::ok_body(html, MimeType::Html).xok()
}
