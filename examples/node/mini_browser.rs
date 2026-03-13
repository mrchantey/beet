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
	commands.spawn((Layout::vertical(), children![
		TuiTextBox::new("url", &url),
		(TuiNodeRenderer::default(), render_on_spawn(url))
	]));
}

fn render_on_spawn(url: String) -> impl Bundle {
	OnSpawn::new_async(async move |entity| {
		// 1. Fetch the URL
		let input_bytes = Request::get(url)
			.send()
			.await
			.unwrap()
			// check for 200 status
			.into_result()
			.await
			.unwrap()
			// get body typed by the content-type header
			.media_bytes()
			.await
			.unwrap();

		entity
			.with_then(move |mut entity| {
				// 2. Parse the media and insert the entity tree for rendering
				MediaParser::new()
					.parse(ParseContext::new(&mut entity, &input_bytes))
					.unwrap();

				// 3. Mark widget as changed to trigger rerender
				entity.get_mut::<TuiWidget>().unwrap().set_changed();
			})
			.await;

		Ok(())
	})
}
