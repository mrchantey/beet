//! The [`MediaRenderer`] will select the best renderer based on a
//! list of accepted [`MediaType`].
//!
//!
//! ```sh
//! # ansi-term
//! cargo run --example media_renderer -- --media-type text/ansi-term
//! # html
//! cargo run --example media_renderer -- --media-type text/html
//! # markdown
//! cargo run --example media_renderer -- --media-type text/markdown
//! ```
use beet::prelude::*;

fn main() {
	let mut world = World::new();
	let entity = world.spawn_empty().id();
	let md_bytes = MediaBytes::markdown(MARKDOWN);

	// 1. Load the markdown into ecs
	MarkdownParser::new()
		.parse(ParseContext::new(&mut world.entity_mut(entity), &md_bytes))
		.unwrap();

	let output = world
		.run_system_once(move |walker: NodeWalker| {
			// 2. Get the requested media type
			let args = CliArgs::parse_env();
			let media_type: MediaType = args
				.params
				.get_multikey(["media-type", "t"])
				.map(|val| val.parse().unwrap())
				.unwrap_or(MediaType::AnsiTerm);

			// 3. Render to the requested media type
			MediaRenderer::default()
				.render(
					&RenderContext::new(entity, &walker)
						.with_accepts(vec![media_type]),
				)
				.unwrap()
				.to_string()
		})
		.unwrap();
	println!("{output}");
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
