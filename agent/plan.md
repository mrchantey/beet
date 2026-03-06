# beet_node -> Markdown

Our html integration is shaping up `crates/beet_node/src/parse/html`, `crates/beet_node/src/render/html`.

Time to implement the markdown parser! Pay attention, these two parsers are linked and we'll need careful planning to ensure the implementation is beautiful and not full of duplicated code and types.

- `pulldown-cmark` emits opaque HtmlBlock events, which we will need to parse with our html parser.
- just like html, tracking the spans is very important. we may need to adjust the Node
- the `MarkdownParser` will need a {html: HtmlParser} type to allow for options.
- the `markdown_parser` feature must pull in the `html_parser` feature
- We should also add a flag to the html parser: `markdown`, which will parse html text nodes as markdown.
- we should maximally enable pulldown-cmark features like GFM etc
- we'll need to also parse frontmatter, both toml and yaml flavors, lets write our own parsers for this, and parse into bevy_reflect DynamicStruct. the Frontmatter type should be added to the root when present

```rust
#[derive(Component)]
pub struct Frontmatter{
	value: DynamicStruct
}
```
