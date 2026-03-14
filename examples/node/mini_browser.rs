//! An ittie bittie web browser demonstrating the parsing and rendering capabilities of beet.
//!
//! This demo parses html and markdown only, SPAs and css/js heavy sites need not apply
//!
//! ```sh
//! cargo run --example mini_browser --features _mini_browser -- https://wikipedia.org
//! ```
use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((TuiPlugin::default(), AsyncPlugin::default()))
		.add_systems(Startup, setup)
		.run();
}

fn setup(mut commands: Commands) {
	let args = CliArgs::parse_env();
	let url = args
		.path
		.first()
		.cloned()
		.unwrap_or_else(|| "http://example.com".to_string());

	let navigator = commands.spawn(Navigator::new(url.clone())).id();

	commands.spawn((Layout::vertical(), children![
		TuiTextBox::new("url", &url),
		(
			// listens for responses delivered by Navigator
			RenderedBy(navigator),
			// parses the RenderMedia observer into the entity
			MediaParser::default(),
			// renders this entity on a NodeParsed event,
			// triggering a Changed<TuiWidget> which results in
			// a ratatui refresh
			TuiNodeRenderer::default(),
		)
	]));
}
