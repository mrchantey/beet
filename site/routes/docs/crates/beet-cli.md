+++
title = "beet-cli"
+++

# beet-cli

`beet-cli` is the `beet` command-line tool, and it is itself a beet app. Rather than treating the CLI as a special case, every command is an [`Action`] served as a route on a [beet_router](/docs/crates/beet_router) backed by a `CliServer`. So `beet --help` lists the routes and `beet <command>` dispatches one, the same dispatch a beet HTTP server would use. The command line is just another interface over the same routing tree.

| Command | Description |
|---|---|
| `run-wasm` | Cargo runner for `wasm32-unknown-unknown` targets |
| `build-wasm` | Build a wasm module and its bindings |
| `export-pdf` | Render a route to PDF |
| `qrcode` | Generate a QR code (`qrcode` feature) |

```sh
cargo install beet-cli
beet --help
```
