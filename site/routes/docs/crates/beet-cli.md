+++
title = "beet-cli"
+++

# beet-cli

`beet-cli` is the `beet` command-line tool, and it is itself a beet app. Rather than treating the CLI as a special case, every command is an [`Action`] served as a route on a [beet_router](/docs/crates/beet_router) backed by a `CliServer`. So `beet --help` lists the routes and `beet <command>` dispatches one, the same dispatch a beet HTTP server would use. The command line is just another interface over the same routing tree.

| Command | Description |
|---|---|
| `serve` | Serve a no-code site over http/tui/ssh |
| `run-wasm` | Cargo runner for `wasm32-unknown-unknown` targets |
| `build-wasm` | Install the browser binary (`assets/wasm/beet.wasm`) |
| `export-static` | Write a site's static `dist/` |
| `export-pdf` | Render a route to PDF |
| `qrcode` | Generate a QR code (`qrcode` feature) |

```sh
cargo binstall beet-cli    # or cargo install beet-cli --all-features
beet --help
```

Running an entry is the zero-command path: `beet` discovers a `main.bsx` by walking the cwd and its ancestors, `--main` names an entry file or a directory holding one, and `--features=a,b` verifies the installed binary was compiled with those cargo features (entries also declare their own requirements with `<CrateCheck>`):

```sh
beet --main=examples/hello
beet --main=examples/perceive_act/main-v1.bsx
```
