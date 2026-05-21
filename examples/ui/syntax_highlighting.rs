//! Parses a markdown document containing several fenced code blocks and
//! renders it to styled ANSI terminal output with tree-sitter syntax
//! highlighting.
//!
//! ```sh
//! cargo run --example syntax_highlighting --features "syntax_highlighting,markdown"
//! ```
use beet::prelude::*;

fn main() {
	// 1. Configure an app with `StylePlugin`, which registers the
	// `SyntaxHighlighting` resource and the `apply_syntax_highlighting`
	// system in the `PostParseTree` schedule.
	let mut app = App::new();
	app.add_plugins(StylePlugin);
	let entity = app.world_mut().spawn_empty().id();

	// 2. Parse the markdown source into the entity tree. The parser runs
	// `PostParseTree` automatically once the tree is built.
	let bytes = MediaBytes::new_markdown(MARKDOWN);
	MarkdownParser::new()
		.parse(ParseContext::new(
			&mut app.world_mut().entity_mut(entity),
			&bytes,
		))
		.unwrap();

	// 3. Render. The charcell-backed ANSI renderer paints the resolved
	// `VisualStyle` of each span; the `hl-*` classes emitted during parsing
	// resolve to foreground colours via the default syntax theme registered
	// by `StylePlugin`.
	let output = AnsiTermRenderer::new()
		.with_clear_on_render(false)
		.render(&mut RenderContext::new(entity, app.world_mut()))
		.unwrap()
		.to_string();
	println!("{output}");
}

const MARKDOWN: &str = r#"
# Syntax Highlighting Showcase

Below are a few code samples to demonstrate tree-sitter powered
highlighting for the languages bundled with `beet_ui`.

## Rust

```rust
/// Compute the n-th fibonacci number.
fn fibonacci(n: u32) -> u64 {
    let mut a: u64 = 0;
    let mut b: u64 = 1;
    for _ in 0..n {
        let next = a + b;
        a = b;
        b = next;
    }
    a
}

fn main() {
    println!("fib(10) = {}", fibonacci(10));
}
```

## JavaScript

```javascript
// Async hello world.
async function greet(name) {
    const message = `Hello, ${name}!`;
    await new Promise(r => setTimeout(r, 100));
    return message;
}

greet("world").then(console.log);
```

## JSON

```json
{
  "name": "beet_ui",
  "version": "0.0.9",
  "features": ["style", "syntax_highlighting"]
}
```

## HTML

```html
<section class="card">
    <h1>Tree-sitter</h1>
    <p>Beautifully highlighted code, everywhere.</p>
</section>
```
"#;
