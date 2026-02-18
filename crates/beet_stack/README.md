# Beet Stack

An opinionated, interface-agnostic application framework inspired by the HyperCard stack/card metaphor.

### Cards

Cards are routable content tools, similar to pages in a website or files in a filesystem. Each card is a tool that accepts a path and a content handler, delegating rendering to the nearest [`RenderToolMarker`] entity.

Cards may contain content, tools, and nested cards.

```rust
use beet_stack::prelude::*;
use beet_core::prelude::*;

let root = (
	default_router(),
	children![
		card("about", || Paragraph::with_text("About page")),
		card("settings", || Paragraph::with_text("Settings")),
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


let output: i32 = AsyncPlugin::world()
	.spawn(add_tool())
	.call_blocking((1, 2))
	.unwrap();

assert_eq!(output, 3);
```

### RouteTree

The `RouteTree` collects all tools in an entity hierarchy into a validated routing tree. It is automatically inserted on the root ancestor whenever routes are registered. Cards register as tools with `is_card: true` on the `ToolNode`.

```text
RouteTree
  /about [card]         <- card tool
  /settings [card]      <- card tool
  /increment            <- tool
    input:  ()
    output: i64
```

### Content

Static or dynamic information presented to the user, like text or images. Content uses semantic markers (`Heading1`, `Paragraph`, `Important`, `Emphasize`, `Code`, `Quote`, `Link`) that are rendered differently depending on the interface.

### Render Tools

Render tools determine how card content is displayed. Different servers provide different render tools:

- **Markdown** (`markdown_render_tool`): spawns content, renders to markdown, despawns. Used by CLI and REPL servers.
- **TUI**: manages stateful card display in the terminal.

Render tools belong on the server, not the router. CLI and REPL servers
include `markdown_render_tool` automatically.

### Interfaces

Interfaces determine how cards, content, and tools are presented and interacted with:

```sh
> my_app
# prints help for root: subcommands: `foo`
> foo
# renders foo via the render tool, sets current card to `foo`
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
- **`stack`** - `Card`, `card()`, `file_card()`, `CardQuery`, `RenderRequest`, built-in tools
- **`content`** - Semantic text content and markers
- **`document`** - Structured data storage with field-level access
- **`interface`** *(feature-gated)* - `Interface`, `route_tool()`, help, markdown rendering, render tools
