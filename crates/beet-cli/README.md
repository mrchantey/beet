# beet-cli

The `beet` command-line interface: a game engine for tools.

Like a game engine, the binary is unopinionated. It links a library of
capabilities (registered reflect types) but ships zero behaviour, so opening it
does nothing until you supply an entry. On startup it discovers `main.bsx` (or
`main.json` / `main.ron`) by walking the cwd's ancestors, with `--main=<path>` as
an override. It parses argv once into a request the loaded tree consumes, builds
the entry through the unified loader, and lets its load-lifecycle verb run: a
`ScriptEntry` script runs and exits, or a `BootOnLoad` server entry fans the
request out to its servers. A one-shot streams its response and exits; a
long-running server parks its boot call to keep the process alive.

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
