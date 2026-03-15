# Beet Stack

An interface-agnostic application framework inspired by the HyperCard stack/card paradigm.

```rust,ignore
fn warm_greeting() -> impl Bundle {
	card("/warm-greeting", move || {
		mdx!("hello **world**")
	})
}
```

Beet is an extension of the bevy engine.

```rust,ignore
App::new()
	.add_plugins((MinimalPlugins, StackPlugin))
	.add_systems(Startup, |mut commands: Commands| {
		commands.spawn((http_server(5000), warm_greeting()));
	})
	.run()
```

### Is it a web framework?

Beet is centered around the Bevy ECS model, supporting many domains including the web:

**Servers**
- `http`
- `discord`
- `cli`
- `repl`

**Parsers & Renderers**
- `html`: Static Site Generation 
- `dom`: Single Page Applications 
- `ratatui`: Terminal UI
- `bevy_ui`: Native and spatial applications

These can be combined to create frameworks for applications across domains like web, games, robotics.

> Got an integration? create a PR adding your crate to the list!

**Endpoints** 

Endpoints accept and return arbitary payloads.

```rust,ignore

#[tool]
fn add(a:i32, b:i32) -> i32 {
	a + b
}
```
**Cards**

Cards perform some operation on an entity, for example populating it with text content, then provide it to the server for rendering, usually according to the request `Accept` header.

### Everything is a tool

Both endpoints and cards are defined as tools, an **entities as functions** pattern used throughout beet.

Tools may be one of three types:
- Pure Functions: great for middleware and error handling
- Bevy Systems: useful when world access is required
- Async Systems: great for IO tasks or calling other tools


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

#[tool]
fn add_tool(a: i32, b: i32) -> i32 { a + b }

let output: i32 = AsyncPlugin::world()
	.spawn(add_tool.into_tool())
	.call_blocking((1, 2))
	.unwrap();

assert_eq!(output, 3);
```

### RouteTree

The `RouteTree` collects all tools in an entity hierarchy into a validated routing tree. It is automatically inserted and updated on the root ancestor whenever a tool or card is added.

```text
RouteTree
  /about [card]
  /settings [card]
  /increment
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
- **`tools`** - `Tool`, `ToolMeta`, `#[tool]` macro (via `beet_tool`)
- **`stack`** - `CardTool`, `card()`, `file_card()`, `CardQuery`, `RenderRequest`, built-in tools
- **`content`** - Semantic text content and markers
- **`document`** - Structured data storage with field-level access
- **`interface`** *(feature-gated)* - `Interface`, `route_tool()`, help, markdown rendering, render tools
