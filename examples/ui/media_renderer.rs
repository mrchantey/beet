//! [`RenderTargets`] selects the render target for a request accepting a
//! [`MediaType`]. Use cli args to specify the output
//!
//! With the `markdown` feature the markdown is parsed into structured
//! nodes; without it the [`MediaParser`] falls back to plain text.
//!
//! ```sh
//! # ansi-term (default)
//! cargo run --example media_renderer -- --media-type text/ansi-term
//! # html
//! cargo run --example media_renderer -- --media-type text/html
//! # markdown
//! cargo run --example media_renderer -- --media-type text/markdown
//! ```
use beet::prelude::*;

fn main() {
	let mut world = RenderPlugin.into_world();
	let mut entity = world.spawn_empty();
	let md_bytes = MediaBytes::new_markdown(MARKDOWN);

	// 1. Load the markdown into ecs — falls back to plain text without
	// the `markdown_parser` feature.
	MediaParser::new()
		.parse(ParseContext::new(&mut entity, &md_bytes))
		.unwrap();

	// 2. Get the requested media type
	let media_type = CliArgs::parse_env()
		.params
		.get_multikey(["media-type", "t"])
		.map(|val| val.parse().unwrap())
		.unwrap_or(MediaType::AnsiTerm);

	// 3. Render as the answer to a request accepting that media type
	let entity = entity.id();
	let output = RenderTargets::render(
		&mut world,
		entity,
		&RequestParts::default().with_accept(media_type),
	)
	.unwrap()
	.to_string();
	cross_log!("{output}");
}

const MARKDOWN: &str = r#"
# All about crystals

Crystals are people like you and me.
They come in all shapes and sizes and when you boink them with a hammer they might break.
There are **only three kinds** of crystals in the world:
- little ones
- big ones
- weird ones

> *I tried eating one once but it didn't taste very nice*
>
> —— Some fool

## Instructions

If you find a crystal put it in your pocket.
But if it decides to go off wandering thats ok, sometimes they like to do that.

[Find out more](https://www.gutenberg.org/cache/epub/14209/pg14209-images.html)
"#;
