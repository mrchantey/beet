//! Fetch a URL and render it in the terminal using [`MediaRenderer`].
//!
//! Combines the HTTP client with [`MediaParser`] and [`MediaRenderer`]
//! to load remote content into ECS, then render it with the chosen
//! media type. The `content-type` response header is used to select
//! the parser automatically via [`MediaParser`].
//!
//! ```sh
//! # default: fetch example.com, render as ansi-term
//! cargo run --example pretty_curl --features _pretty_curl
//! # specify a url
//! cargo run --example pretty_curl --features _pretty_curl -- http://example.com
//! # render as html
//! cargo run --example pretty_curl --features _pretty_curl -- http://example.com --media-type=text/html
//! # render as markdown
//! cargo run --example pretty_curl --features _pretty_curl -- http://example.com --media-type=text/markdown
//! # render as plain text
//! cargo run --example pretty_curl --features _pretty_curl -- http://example.com --media-type=text/plain
//! ```
use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((
			#[cfg(not(feature = "tui"))]
			MinimalPlugins,
			#[cfg(feature = "tui")]
			TuiPlugin::default(),
			AsyncPlugin::default(),
		))
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

		let render_media_type: Option<MediaType> = args
			.params
			.get_multikey(["media-type", "t"])
			.map(|val| val.parse().unwrap());

		// 1. Fetch the URL
		let media_bytes = Request::get(&url)
			.send()
			.await
			.unwrap()
			// check for 200 response
			.into_result()
			.await
			.unwrap()
			// get body
			.media_bytes()
			.await
			.unwrap();

		// 2. Parse the response body into ECS and render it
		let media_type = render_media_type.unwrap_or(MediaType::AnsiTerm);

		world.with(move |world: &mut World| {
			let mut entity = world.spawn_empty();

			MediaParser::new()
				.parse(ParseContext::new(&mut entity, &media_bytes))
				.unwrap();

			// 3. Render to the requested media type
			#[allow(unused)]
			let output = MediaRenderer::default()
				.run(&mut entity, vec![media_type])
				.unwrap()
				.to_string();
			#[cfg(not(feature = "tui"))]
			println!("{output}");
		});

		#[cfg(not(feature = "tui"))]
		world.write_message(AppExit::Success);
	});
}
