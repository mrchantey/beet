//! An ittie bittie web browser demonstrating the parsing and rendering capabilities of beet.
//!
//! This demo parses html and markdown only, SPAs and css/js heavy sites need not apply
//!
//! ```sh
//! cargo run --example mini_browser --features _mini_browser
//! ```
use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((TuiPlugin::default(), AsyncPlugin::default()))
		.add_systems(Startup, fetch_and_render)
		.run();
}

fn fetch_and_render(mut async_commands: AsyncCommands) {
	async_commands.run(|world| async move {
		let args = CliArgs::parse_env();
		let url = args
			.path
			.first()
			.cloned()
			.unwrap_or_else(|| "http://example.com".to_string());

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

		world.with(move |world: &mut World| {
			let mut entity = world.spawn(TuiNodeRenderer::default());
			// let mut entity = world.spawn_empty();

			// 2. Parse the response body into ECS and render it
			MediaParser::new()
				.parse(ParseContext::new(&mut entity, &input_bytes))
				.unwrap();
			// TuiNodeRenderer::default()
			// 	.run(&mut entity, vec![MediaType::Ratatui])
			// 	.unwrap()
			// 	.to_string();
		});
	});
}
