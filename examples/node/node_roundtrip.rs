use beet::prelude::*;

fn main() {
	let mut world = World::new();
	let entity = world.spawn_empty().id();
	let md_bytes = MediaBytes::from_str(MediaType::Markdown, MARKDOWN);
	MarkdownParser::new()
		.parse(ParseContext::new(&mut world.entity_mut(entity), &md_bytes))
		.unwrap();

	let output = world
		.run_system_once(move |walker: NodeWalker| {
			let args = CliArgs::parse_env();

			// If --media-type or -t is provided, use MediaRenderer.
			// Otherwise default to AnsiTermRenderer for terminal output.
			if let Some(type_str) = args
				.params
				.get("media-type")
				.or_else(|| args.params.get("t"))
			{
				let media_type: MediaType = type_str.parse().unwrap();
				let cx = RenderContext::new(entity, &walker)
					.with_accepts(vec![media_type.clone()]);
				MediaRenderer::new(media_type)
					.render(&cx)
					.unwrap()
					.to_string()
			} else {
				let cx = RenderContext::new(entity, &walker);
				AnsiTermRenderer::new().render(&cx).unwrap().to_string()
			}
		})
		.unwrap();
	println!("{output}");
}

const MARKDOWN: &str = r#"
# All about crystals

Crystals are people like you and me.
They come in all shapes and sizes and when you boink them with a hammer they might break.
There are **three** kinds of crystals in the world:
- little ones
- big ones
- weird ones


## Instructions
> *I tried eating one once but it didn't taste very nice*
>
> — Some fool

If you find a crystal put it in your pocket.
But if it decides to go off wandering thats ok, sometimes they like to do that.

## More information

Find out more at [The Kybalion](https://www.gutenberg.org/cache/epub/14209/pg14209-images.html)
"#;
