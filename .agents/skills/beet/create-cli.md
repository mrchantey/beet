# Create a beet CLI

A beet CLI is just a router: a [`CliServer`] reads the process args as a
[`Request`], the [`router`] dispatches it to a child route, and the route's
response is written to stdout. There is no bespoke arg-parsing layer — a command
is a route, its flags are request params, and `--help` is router middleware.

The canonical example is the `beet` CLI itself (`crates/beet-cli`). Read it
alongside this skill; everything below is drawn from it.

## 1. The app

```rust
use beet::prelude::*;
use beet_cli::prelude::*;

fn main() -> AppExit {
	App::new()
		.add_plugins((MinimalPlugins, LogPlugin::default(), ClientAppPlugin))
		.add_systems(Startup, setup)
		.run()
}

/// Spawns the CLI server with every command wired as a route.
fn setup(mut commands: Commands) {
	commands
		.spawn((CliServer::default(), default_router()))
		.with_children(|parent| {
			parent.spawn(exchange_route("build-wasm", BuildWasm));
			parent.spawn(exchange_route("export-pdf", ExportPdf));
			parent.spawn(exchange_route("run-wasm/*args", RunWasm));
		});
}
```

- `CliServer` turns `beet <args>` into a request and streams the response to
  stdout, mapping a non-OK status to a non-zero exit code.
- `default_router()` bundles the route lookup plus the `RequestLogger`,
  `HelpHandler` and `NavigateHandler` middleware and the default app routes;
  spawn your own routes as `children` of the same entity.
- Each `exchange_route(path, Action)` is one command. The `path` is matched
  against the args, ie `beet build-wasm` hits the `build-wasm` route.

## 2. A command

A command is an `#[action]` async fn taking [`RequestParts`] and returning
`Result<String>`. The string is the response body.

```rust
#[action]
#[derive(Component)]
#[require(ParamsPartial = ParamsPartial::new::<QrCodeParams>())]
pub async fn QrCode(parts: RequestParts) -> Result<String> {
	let params = parts.params().parse_reflect::<QrCodeParams>()?;
	let output = params.output.as_deref().unwrap_or("qrcode.png");
	// ..
	Ok(format!("wrote qr code to {output}"))
}
```

## 3. Params: parse, never hand-roll

Define a `Reflect` struct for the flags. Do NOT pull values out one by one with
`parts.get_param("..")` — that skips validation, defaults, and the help
listing. Instead parse the whole struct in one call:

```rust
/// Request params for the [`QrCode`] command, surfaced in `--help`.
#[derive(Reflect, Default)]
#[reflect(Default)]
struct QrCodeParams {
	/// The text/url to encode.
	#[reflect(@RequiredField)]
	input: String,
	/// The output file path, defaults to `qrcode.png`.
	output: Option<String>,
}
```

```rust
// BAD — manual, unvalidated, invisible to --help
let input = parts.get_param("input").ok_or_else(|| bevyhow!(".."))?;

// GOOD — one typed parse
let params = parts.params().parse_reflect::<QrCodeParams>()?;
```

Rules for the params struct:

- Always derive `Default` and add `#[reflect(Default)]`. Missing flags fall back
  to the struct's `Default`, so parsing partial args just works. Without it
  `parse_reflect` cannot build the struct from an incomplete arg set.
- Field names are snake_case; the CLI flag is the kebab-case form, ie
  `out_dir` ↔ `--out-dir`. The normalisation is automatic.
- `bool` → a flag (`--release`). `Option<T>` → optional. A bare field → uses
  the struct `Default` when absent.
- Mark a field required with `#[reflect(@RequiredField)]`; parsing errors if it
  is missing. Prefer this over a manual presence check.
- For a default that is not the type's own default (ie `a` defaults to `2`),
  give the struct a custom `Default` impl and keep the field bare:
  ```rust
  #[derive(Reflect)]
  #[reflect(Default)]
  struct CallAddParams { a: i32, b: i32 }
  impl Default for CallAddParams {
  	fn default() -> Self { Self { a: 2, b: 3 } }
  }
  ```
- Supported field types: `bool`, `String`, `Option<String>`, `Vec<String>`, the
  numeric primitives, and nested/newtype structs. `Option<numeric>` is not
  supported — use a bare numeric with a custom `Default` instead.

## 4. `--help` is free

`#[require(ParamsPartial = ParamsPartial::new::<QrCodeParams>())]` registers the
param metadata on the route. The router's `HelpHandler` intercepts `--help`,
walks the [`RouteTree`], and renders the available routes and their params. Doc
comments on the params fields become the flag descriptions, so document them.
`beet --help` lists everything; `beet qrcode --help` scopes to that subtree.

## 5. Greedy routes and forwarding args

A trailing `*name` segment captures the rest of the args greedily, eg the
`run-wasm/*args` cargo runner. To rebuild a forwardable arg vector from the
request use [`RequestParts::unparse_cli_args`] — it returns every path segment
as a positional followed by params as `--key`/`--key=value`:

```rust
pub async fn RunWasm(parts: RequestParts) -> Result<String> {
	// rebuilds `[run-wasm, <binary>, ..forwarded]`; skip the command segment
	// consumed by the route, pop the binary, forward the rest to the module.
	let mut args = parts.unparse_cli_args().into_iter().skip(1);
	let exe_path = args
		.next()
		.ok_or_else(|| bevyhow!("usage: beet run-wasm <binary-path> [args..]"))?;
	run_wasm(Path::new(&exe_path), args.collect()).await?;
	Ok(String::new())
}
```

## 6. Output and content negotiation

The body is rendered per the `--accept` header (default: ansi-term, then text,
markdown, json). A plain `Ok(String)` prints as text; a scene route (eg
`render_action::async_route`) renders through the beet_ui pipeline, so `--accept=text/html`
yields HTML and the default yields styled terminal output. Rendering requires
`RouterPlugin` (pulled in by `ClientAppPlugin`), which registers the charcell
render pipeline.

## Reference

- `crates/beet-cli/src/main.rs` — app + route wiring
- `crates/beet-cli/src/commands/qrcode.rs` — params + `parse_reflect`
- `crates/beet-cli/src/commands/run_wasm.rs` — greedy route + `unparse_cli_args`
- `examples/file_based_routes/main.rs` — a CLI/HTTP app sharing one router
