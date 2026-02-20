# Plan


## `crates/beet_stack`

Lets get our renderer examples working. it looks like the render tools are not getting picked up: `cargo run --example cli --features=stack`

Ensure the render tool is inserted by the server, ie `cli_server`, `tui_server`..

### Features

- `tui_render_tool.rs`: stateful renderers are only partially implemented. one way to see the current status is to run the examples: `cargo run --example tui --features=tui`.

### Chores

- rename Card [`Card`] to CardTool.