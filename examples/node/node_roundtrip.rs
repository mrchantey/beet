use beet::prelude::*;


fn main() {
	let mut world = World::new();
	let entity = world.spawn_empty().id();
	MarkdownParser::new()
		.parse(
			&mut world.entity_mut(entity),
			MARKDOWN.as_bytes().to_vec(),
			None,
		)
		.unwrap();
	let output = world
		.run_system_once(move |walker: NodeWalker| {
			let args = CliArgs::parse_env();
			match args.params.get("format").map(|s| s.as_str()) {
				Some("html") => {
					let mut renderer = HtmlRenderer::new();
					walker.walk(&mut renderer, entity);
					renderer.into_string()
				}
				Some("markdown") | Some("md") => {
					let mut renderer = MarkdownRenderer::new();
					walker.walk(&mut renderer, entity);
					renderer.into_string()
				}
				Some("ansi") | None => {
					// default to ansi
					let mut renderer = AnsiTermRenderer::new();
					walker.walk(&mut renderer, entity);
					renderer.into_string()
				}
				Some(other) => {
					panic!("Unknown format: {other}");
				}
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
> — Some fool

If you find a crystal put it in your pocket.
But if it decides to go off wandering thats ok, sometimes they like to do that.

## More information

Find out more at [The Kybalion](https://www.gutenberg.org/cache/epub/14209/pg14209-images.html)
"#;
