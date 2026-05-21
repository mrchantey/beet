//! Parses a markdown document containing several fenced code blocks and
//! renders it to styled ANSI terminal output with tree-sitter syntax
//! highlighting.
//!
//! ```sh
//! cargo run --example syntax_highlighting --features "syntax_highlighting,markdown"
//! ```
use beet::prelude::*;

fn main() {
	let mut world = World::new();
	let entity = world.spawn_empty().id();

	// 1. Configure a markdown parser with syntax highlighting enabled.
	let mut parser = MarkdownParser::new();
	parser.config.syntax_highlighting =
		Some(SyntaxHighlighting::with_defaults());

	// 2. Parse the markdown source into the entity tree.
	let bytes = MediaBytes::new_markdown(MARKDOWN);
	parser
		.parse(ParseContext::new(&mut world.entity_mut(entity), &bytes))
		.unwrap();

	// 3. Render. The ANSI renderer resolves each span's `hl-*` class to a
	// styled foreground colour using the default syntax highlight palette.
	let output = AnsiTermRenderer::new()
		.with_clear_on_render(false)
		.with_syntax_highlighting()
		.render(&mut RenderContext::new(entity, &mut world))
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
