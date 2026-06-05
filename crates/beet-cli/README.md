# beet-cli

The `beet` command-line interface: build, serve and run-wasm helpers for beet apps.

The CLI is itself a beet app. Every command is an [`Action`] served as a route on a [`CliServer`]-backed router, so `beet --help` lists the commands and `beet <command>` dispatches one.

| Command | Description |
|---------|-------------|
| `run-wasm` | Cargo runner for `wasm32-unknown-unknown` targets |
| `build-wasm` | Build a wasm module and its bindings |
| `export-pdf` | Render a route to PDF |
| `qrcode` | Generate a QR code (`qrcode` feature) |

```sh
cargo install beet-cli
beet --help
```
