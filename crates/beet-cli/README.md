# beet-cli

The `beet` command-line interface: a scene runner whose commands are loaded from a `beet.json`.

The CLI is itself a beet app. Like a game engine pressing play with no scene loaded, the bare binary does nothing. On startup it looks for a `beet.json` in the cwd: absent, it prints a welcome message and exits; present, it loads that scene, which supplies the actual CLI as routes on a [`CliServer`]-backed router, then watches the file for live reloads. So `beet --help` lists the loaded commands and `beet <command>` dispatches one.

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
