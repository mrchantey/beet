//! The [`MediaRenderer`] will select the best renderer based on a
//! list of accepted [`MediaType`].
use beet::prelude::*;

fn main() {
	let mut world = World::new();
	let entity = world.spawn_empty().id();
	let md_bytes = MediaBytes::markdown(MARKDOWN);
	MarkdownParser::new()
		.parse(ParseContext::new(&mut world.entity_mut(entity), &md_bytes))
		.unwrap();

	let output = world
		.run_system_once(move |walker: NodeWalker| {
			let args = CliArgs::parse_env();

			let media_type: MediaType = args
				.params
				.get("media-type")
				.or_else(|| args.params.get("t"))
				.map(|val| val.parse().unwrap())
				.unwrap_or(MediaType::AnsiTerm);

			let cx = RenderContext::new(entity, &walker)
				.with_accepts(vec![media_type]);
			MediaRenderer::default().render(&cx).unwrap().to_string()
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
