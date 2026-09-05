# beet-cli

The `beet` command-line interface: a game engine for tools.

Like a game engine, the binary is unopinionated. It links a library of
capabilities (registered reflect types) but ships zero behaviour, so opening it
does nothing until you supply an entry. On startup it discovers `main.bsx` (or
`main.json` / `main.ron`) by walking the cwd's ancestors, with `--main=<path>` as
an override. It parses argv once into a request the loaded tree consumes, builds
the entry through the unified loader, and lets its load verb (`CallOnReady`) call
the entry's own action with that request: a script runs and exits, or a server
entry fans the request out to the servers it declares, each dispatching down into
its router child. A one-shot streams its response and exits; a long-running server
parks its boot call to keep the process alive.

There is no built-in command, route, host, or retained state. The dev commands
below are capabilities the repo's own `main.bsx` wires as routes, so
`beet run-wasm <module>` discovers that entry and dispatches the route. A no-code
site (eg `examples/bsx_site`) declares its own servers and routes in markup, so
`cd examples/bsx_site && beet --server=http` serves it.

| Capability | Description |
|------------|-------------|
| `run-wasm` | Cargo runner for `wasm32-unknown-unknown` targets |
| `build-wasm` | Build a wasm module and its bindings |
| `check` | Lint a no-code site's routes |
| `export-static` | Render a no-code site to its `dist/` |
| `export-pdf` | Render a route to PDF |
| `s3-sync` | Sync a directory between the local filesystem and S3 |
| `qrcode` | Generate a QR code (`qrcode` feature) |

```sh
# links the capabilities; the repo's main.bsx wires the dev commands as routes
cargo install --path crates/beet-cli

beet run-wasm <module.wasm>               # eg the wasm test runner
cd examples/bsx_site && beet --server=http
```

## Entries

`--main` accepts an entry file (`--main=examples/hello/main.bsx`) or a directory probed for `main.bsx` (`--main=examples/hello`); with no `--main`, discovery walks the cwd and its ancestors.

An entry loads through its **repo store**, the app's one canonical store (`--repo` / `BEET_REPO` picks the backend, defaulting to a filesystem store at the entry directory; `--repo=s3://<bucket>` is what a deployed box launches with). The built root carries it marked `RepoStore`, so everything below resolves content by ancestry and a second repo store anywhere in the world is an error.

An entry that mounts paths outside its own directory declares `<RepoRoot src="../.."/>` (there is no `--root` flag): `src` names a position relative to the entry's location *in its repo store*, not a filesystem directory, so an fs store re-roots at the resolved ancestor while a self-rooted store (a bucket, browser storage) takes a key-prefix view and fails loudly when the root escapes the store. Live reload watches the store's local root when it has one (a self-rooted store watches nothing), and command outputs (`dist/`, `site.pdf`) land beside the entry deliberately.

An entry declares its required features with `<CrateCheck features={["thread", "sockets"]}/>`, which errors when the running binary lacks them; `beet --features=..` performs the same check from argv. A runnable documented command is therefore plain `beet --main=..`, never carrying `--features`: the entry's own `<CrateCheck>` is the verification mechanism.

## Downstream binaries

The stock `beet` binary resolves the types beet itself registers, so a workspace that names only those runs through it. A workspace that EXTENDS beet — its own `#[action]`s, deploy blocks, reflect components — builds a binary of its own, because no beet build can know those types: an entry naming one warns, marks the entity `UnregisteredTag` and runs a tree with that behaviour simply missing.

Such a binary is a thin `main`, and it depends on the `beet` facade alone. The runner is `beet::launch::app` (`BeetPlugins` + `beet_router`'s `LaunchPlugin`), so nothing about it is this crate's:

```rust,ignore
use beet::prelude::*;

fn main() -> AppExit {
	env_ext::load_dotenv().ok();
	let mut app = launch::app(MyCratePlugin);
	// only the binary knows its own cargo features; an unprefixed `<CrateCheck/>`
	// resolves against this
	app.world_mut()
		.spawn(crate_registration!({ features: ["my-feature"] }).with_skip_prefix());
	app.run()
}
```

It then has the same entry resolution, load and process lifecycle this binary has. What it does NOT get is the dev commands below (`run-wasm`, `build-wasm`, `check`, `export-static`, …), which are `CliCommandsPlugin` and live here — this binary adds them as its own `launch::app` argument, exactly as a downstream one adds its own.

## Development

- when editing rust, run the workspace CLI: `cargo run -p beet-cli --features=feat1,feat2 -- arg1 arg2`
- when editing bsx files, use the installed `beet`: `cargo install --path crates/beet-cli --all-features`

## Browser binaries

Three wasm binaries span the range, built with `just build-wasm-min` (`assets/wasm/beet-min.wasm`, the smallest binary that is still a beet runtime in the browser), `just build-wasm-render` (`assets/wasm/beet-render.wasm`, the windowed wgpu stack in a tab: GPU via WebGPU when the browser grants an adapter, GPU-less otherwise) and `just build-wasm-full` (`assets/wasm/beet-full.wasm`, every feature a browser binary can boot with). Their feature sets are `web_min`/`web_render`/`web_full` in `crates/beet-cli/Cargo.toml`, whose comments carry the remaining exclusions and why; a narrower set in between gets its own artifact rather than widening `min`. `build-wasm` itself is target-agnostic, so package/features/out are always explicit, never defaulted to a beet binary.
