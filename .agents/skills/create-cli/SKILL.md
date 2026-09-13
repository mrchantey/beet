---
name: create-cli
description: Create a beet CLI, ie a CliServer router whose commands are routes, flags are request params and --help is middleware, serializable to a beet.json scene. Use when building or extending a CLI on beet.
---

# Create a beet CLI

A beet CLI is just a router: a [`CliServer`] reads the process args as a [`Request`], the [`router`] dispatches it to a child route, and the route's response is written to stdout. There is no bespoke arg-parsing layer — a command is a route, its flags are request params, and `--help` is router middleware.

The canonical example is the `beet` CLI itself: the `beet` binary (`crates/beet-cli/src/main.rs`) is a bare scene runner that loads its routes from a `beet.json`, and the `default_cli` example (`crates/beet-cli/examples/default_cli.rs`) builds that scene and serializes it. Read them alongside this skill; everything below is drawn from them.

## 1. The app

A CLI is a [`CliServer`] router whose routes are the commands. The commands are reflectable route actions (`#[action(route = "...")]`), so the whole tree can be serialized to a `beet.json` scene and reloaded by the `beet` runner:

```rust
use beet::prelude::*;
use beet_cli::prelude::*;

/// The default `beet` CLI: a CliServer router wiring every built-in command.
fn default_cli(world: &mut World) -> Entity {
	let entity = world.spawn((CliServer::default(), default_router())).id();
	world.entity_mut(entity).with_children(|parent| {
		parent.spawn(BuildWasm);
		parent.spawn(ExportPdf);
		parent.spawn(RunWasm);
	});
	entity
}
```

Each command is spawned as a bare marker: its `#[action(route = "...")]` attribute requires the `PathPartial` + dispatch components, so the route is self-describing and round-trips through world serde with no extra wiring.

- `CliServer` turns `beet <args>` into a request and streams the response to stdout, mapping a non-OK status to a non-zero exit code.
- `default_router()` bundles the route lookup plus the `RequestLogger`, `HelpHandler` and `NavigateHandler` middleware and the default app routes; spawn your own routes as `children` of the same entity.
- Each command marker is one command. The `route` path is matched against the args, ie `beet build-wasm` hits the `build-wasm` route.

## 2. A command

A command is an `#[action(route = "...")]` async fn taking [`RequestParts`] and returning `Result<String>`. The string is the response body. Derive `Reflect` + `#[reflect(Component)]` so the route serializes into a `beet.json` scene.

```rust
#[action(route = "qrcode")]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(ParamsPartial = ParamsPartial::new::<QrCodeParams>())]
pub async fn QrCode(parts: RequestParts) -> Result<String> {
	let params = parts.parse_params::<QrCodeParams>()?;
	let output = params.output.as_deref().unwrap_or("qrcode.png");
	// ..
	Ok(format!("wrote qr code to {output}"))
}
```

## 3. Params: parse, never hand-roll

Define a `Reflect` struct for the flags. Do NOT pull values out one by one with `parts.get_param("..")` — that skips validation and drifts from the help listing. Instead read the whole struct in one call, `RequestParts::parse_params`, which walks the struct exactly as `ParamsPartial` does for `--help`, so what the help says a flag is, the parse enforces:

```rust
/// Request params for the [`QrCode`] command, surfaced in `--help`.
#[derive(Reflect)]
struct QrCodeParams {
	/// The text/url to encode.
	input: String,
	/// The output file path, defaults to `qrcode.png`.
	output: Option<String>,
}
```

```rust
// BAD — manual, unvalidated, invisible to --help
let input = parts.get_param("input").ok_or_else(|| bevyhow!(".."))?;

// GOOD — one typed read
let params = parts.parse_params::<QrCodeParams>()?;
```

Rules for the params struct:

- Field names are snake_case; the CLI flag is the kebab-case form, ie `out_dir` ↔ `--out-dir`. The normalisation is automatic.
- `bool` → a flag (`--release`), `false` when absent. `Option<T>` → optional, `None` when absent. `Vec<T>` → repeatable (`--tag=a --tag=b`), empty when absent. Any other field is required: parsing errors naming the flag when it is missing, and `--help` lists it as required.
- A default that is not "absent" is the handler's, not the struct's: declare `width: Option<u32>` and read `params.width.unwrap_or(1280)`. A bare `width: u32` would make the flag required.
- A value parses through its type's `LiteralParser`, the same table markup attributes use, so a field is typed as what it is: `store: Option<StoreUri>`, `timeout: Duration`, `created: Timestamp`, a numeric primitive, `String`/`SmolStr`. Nested and newtype structs flatten into the parent's flags.
- No `Default` or `#[reflect(Default)]` is needed, the read supplies every field.
- A markup-authored preset that flags override field-by-field reads through `parts.apply_params(&mut preset)` instead, which touches only the flags present (see `BuildWasm`).

## 4. `--help` is free

`#[require(ParamsPartial = ParamsPartial::new::<QrCodeParams>())]` registers the param metadata on the route. The router's `HelpHandler` intercepts `--help`, walks the [`RouteTree`], and renders the available routes and their params. Doc comments on the params fields become the flag descriptions, so document them. `beet --help` lists everything; `beet qrcode --help` scopes to that subtree.

## 5. Greedy routes and forwarding args

A trailing `*name` segment captures the rest of the args greedily, eg the `run-wasm/*args` cargo runner. To rebuild a forwardable arg vector from the request use [`RequestParts::to_cli_args`] then [`CliArgs::into_args`] — every path segment as a positional followed by params as `--key`/`--key=value`:

```rust
#[action(route = "run-wasm/*args")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn RunWasm(parts: RequestParts) -> Result<String> {
	// rebuilds `[run-wasm, <binary>, ..forwarded]`; skip the command segment
	// consumed by the route, pop the binary, forward the rest to the module.
	let mut args = parts.to_cli_args().into_args().into_iter().skip(1);
	let exe_path = args
		.next()
		.ok_or_else(|| bevyhow!("usage: beet run-wasm <binary-path> [args..]"))?;
	run_wasm(Path::new(&exe_path), args.collect()).await?;
	Ok(String::new())
}
```

## 6. Output and content negotiation

The body is rendered per the `--accept` header (default: ansi-term, then text, markdown, json). A plain `Ok(String)` prints as text; a scene route (eg `render_action::async_route`) renders through the beet_ui pipeline, so `--accept=text/html` yields HTML and the default yields styled terminal output. Rendering requires `RouterPlugin` (pulled in by `ClientAppPlugin`), which registers the charcell render pipeline.

## 7. Shipping the CLI as a scene

The `beet` binary is a scene runner: on startup it loads `beet.json` from the cwd, and that scene supplies the `CliServer` and its routes. The `default_cli` example builds the route tree and serializes it via `WorldSerdeSaver`; the runner reloads it with `WorldSerdeLoader`, reconstructing each command's behaviour from its require hooks. The runner must `register_type` every command (see `CliCommandsPlugin`) so the markers deserialize. The scene management (loading, marking, watching, reloading) lives in `crates/beet-cli/src/scene_management`.

## Reference

- `crates/beet-cli/src/main.rs` — the scene runner
- `crates/beet-cli/examples/default_cli.rs` — build + serialize the CLI scene
- `crates/beet-cli/src/scene_management` — load/watch/reload `beet.json`
- `crates/beet-cli/src/commands/qrcode.rs` — params + `parse_params`
- `crates/beet-cli/src/commands/run_wasm.rs` — greedy route + `to_cli_args`
- `examples/rsx_site/src/server.rs` — a router serving typed pages, markdown content and a server action, with the backend selected by build feature
