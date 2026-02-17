# Beet Stack

An opinionated, interface-agnostic application framework inspired by the HyperCard stack/card metaphore.

### Cards

Cards are navigable content units, similar to pages in a website or files in a filesystem. Each card is a route, with the exact rendering behavior determined by the interface.

Cards may contain content, tools, and nested cards.

```rust
use beet_stack::prelude::*;
use beet_core::prelude::*;

let root = (
	card(""), 
	children![
    card("about"),
    card("settings"),
	]
);
```

### Tools

Tools are callable entities with specified input/output types.

```rust
use beet_stack::prelude::*;
use beet_core::prelude::*;

fn add_tool() -> impl Bundle{
	tool(|(a, b): (i32, i32)| -> i32 { a + b })
}


let output: i32 = World::new()
	.spawn(add_tool())
	.call_blocking((1, 2))
	.unwrap();

assert_eq!(output, 3);
```

### RouteTree

The `RouteTree` collects all cards and tools in an entity hierarchy into a validated routing tree. It is automatically inserted on the root ancestor whenever routes are registered.

```text
RouteTree
  / [card]              <- root card
  /about [card]         <- child card
  /settings [card]      <- child card
  /increment            <- tool
    input:  ()
    output: i64
  /help                 <- tool
    output: alloc::string::String
```

Routes are represented as `RouteNode`, which is either a `Card(CardNode)` or `Tool(ToolNode)`. Common accessors like `entity()`, `path()`, and `params()` are available on `RouteNode` regardless of variant.

### Content

Static or dynamic information presented to the user, like text or images. Content uses semantic markers (`Title`, `Paragraph`, `Important`, `Emphasize`, `Code`, `Quote`, `Link`) that are rendered differently depending on the interface.

### Interfaces (wip)

Interfaces determine how cards, content, and tools are presented and interacted with. The `Interface` component tracks the currently active card, enabling REPL-like navigation:

```sh
> my_app
# prints help for root: subcommands: `foo`
> foo
# renders foo as markdown, sets current card to `foo`
> --help
# prints help for `foo`, not the root
```

Planned interfaces:
- `stdio`: Event-driven command-line interface
- `ratatui`: Terminal user interfaces
- `http`: HTTP server interface
- `dom`: Web-based interfaces
- `wgpu`: Bevy's native UI rendering
- `clanker`: LLM tool calls and context trees
- `embedded`: Microcontrollers like the ESP32

## Modules

- **`router`** - `RouteTree`, `RouterPlugin`, route building observers
- **`tools`** - `tool()`, `ToolMeta`, tool handlers
- **`stack`** - `Card`, `card()`, `CardQuery`, built-in tools (arithmetic, field access)
- **`content`** - Semantic text content and markers
- **`document`** - Structured data storage with field-level access
- **`interface`** *(feature-gated)* - `Interface`, `route_tool()`, help, markdown rendering
